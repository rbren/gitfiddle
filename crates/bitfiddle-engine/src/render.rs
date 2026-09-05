//! Graph execution and offline rendering (PRD §5.3, §5.5, §6.8).
//!
//! Offline rendering uses the same graph, merge logic, and module
//! implementations as live playback, with a deterministic clock origin
//! (sample 0) and deterministic seeds from Rack state.

use std::collections::HashMap;
use std::f32::consts::TAU;

use uuid::Uuid;

use crate::document::{AudioDefaultSource, Endpoint, InputState, RackDocument};
use crate::graph::{build_graph, Graph, GraphError};
use crate::merge;
use crate::modules::registry::{builtin_spec, instantiate_builtin, ModuleSpec};
use crate::modules::{DspModule, ProcessCtx};
use crate::signal::{SignalBlock, SignalType, Voice};

pub struct Engine {
    doc: RackDocument,
    graph: Graph,
    specs: HashMap<Uuid, ModuleSpec>,
    instances: HashMap<Uuid, Box<dyn DspModule>>,
    /// Per-Audio-input deterministic default source generators.
    default_sources: HashMap<Endpoint, DefaultSourceState>,
    sample_pos: u64,
    faults: Vec<String>,
}

struct DefaultSourceState {
    kind: AudioDefaultSource,
    noise_state: u64,
    phase: f32,
}

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error(transparent)]
    Graph(#[from] GraphError),
    #[error("unknown module type: {0}")]
    UnknownModuleType(String),
    #[error("WAV write error: {0}")]
    Wav(#[from] hound::Error),
}

impl Engine {
    pub fn new(doc: RackDocument) -> Result<Self, EngineError> {
        let mut specs = HashMap::new();
        for m in &doc.modules {
            let spec = builtin_spec(&m.type_id)
                .ok_or_else(|| EngineError::UnknownModuleType(m.type_id.clone()))?;
            specs.insert(m.id, spec);
        }
        let graph = build_graph(&doc, &specs)?;

        let mut instances = HashMap::new();
        for m in &doc.modules {
            // Merge seed from Audio-input state into parameters for Noise.
            let mut params = m.state.parameters.clone();
            if m.type_id == "app.noise" && !params.contains_key("seed") {
                params.insert("seed".into(), serde_json::json!(1));
            }
            let inst = instantiate_builtin(&m.type_id, &params)
                .ok_or_else(|| EngineError::UnknownModuleType(m.type_id.clone()))?;
            instances.insert(m.id, inst);
        }

        let mut default_sources = HashMap::new();
        for m in &doc.modules {
            for (port, state) in &m.inputs {
                if let InputState::Audio {
                    default_source,
                    seed,
                    ..
                } = state
                {
                    default_sources.insert(
                        Endpoint {
                            module: m.id,
                            port: port.clone(),
                        },
                        DefaultSourceState {
                            kind: *default_source,
                            noise_state: (*seed).max(1),
                            phase: 0.0,
                        },
                    );
                }
            }
        }

        Ok(Engine {
            doc,
            graph,
            specs,
            instances,
            default_sources,
            sample_pos: 0,
            faults: Vec::new(),
        })
    }

    pub fn document(&self) -> &RackDocument {
        &self.doc
    }

    pub fn faults(&self) -> &[String] {
        &self.faults
    }

    /// Process one block; returns the clipped stereo mixdown of every Audio
    /// Output module summed together.
    pub fn process_block(&mut self, sample_rate: u32, frames: usize) -> Vec<[f32; 2]> {
        let ctx = ProcessCtx {
            sample_rate,
            frames,
            start_sample: self.sample_pos,
        };

        // Outputs published by already-executed modules this block.
        let mut published: HashMap<Endpoint, SignalBlock> = HashMap::new();
        let mut mix = vec![[0.0f32; 2]; frames];

        let order = self.graph.execution_order.clone();
        for module_id in order {
            let module_doc = self
                .doc
                .modules
                .iter()
                .find(|m| m.id == module_id)
                .expect("module in execution order")
                .clone();
            if module_doc.bypassed {
                // v2 bypass for built-ins with declared audio route: copy in->out.
                let spec = &self.specs[&module_id];
                let has_audio_in = spec.inputs.iter().any(|p| p.signal == SignalType::Audio);
                let has_audio_out = spec.outputs.iter().any(|p| p.signal == SignalType::Audio);
                if has_audio_in && has_audio_out {
                    let in_port = spec
                        .inputs
                        .iter()
                        .find(|p| p.signal == SignalType::Audio)
                        .unwrap()
                        .id
                        .clone();
                    let out_port = spec
                        .outputs
                        .iter()
                        .find(|p| p.signal == SignalType::Audio)
                        .unwrap()
                        .id
                        .clone();
                    let block =
                        self.resolve_input(&module_doc, &in_port, &published, sample_rate, frames);
                    published.insert(
                        Endpoint {
                            module: module_id,
                            port: out_port,
                        },
                        block,
                    );
                    continue;
                }
            }

            // Resolve every declared input through the normative pipeline.
            let spec = self.specs[&module_id].clone();
            let mut resolved: HashMap<String, SignalBlock> = HashMap::new();
            for p in &spec.inputs {
                let block = self.resolve_input(&module_doc, &p.id, &published, sample_rate, frames);
                resolved.insert(p.id.clone(), block);
            }

            let instance = self.instances.get_mut(&module_id).unwrap();
            let mut outputs = instance.process(&ctx, &resolved);

            // NaN/inf sanitation at the module boundary (PRD §5.1).
            for (port, block) in outputs.iter_mut() {
                if block.sanitize() {
                    self.faults.push(format!(
                        "{}.{port}: non-finite output replaced",
                        module_doc.name
                    ));
                }
            }

            if module_doc.type_id == "app.audio_output" {
                if let Some(SignalBlock::Audio(voices)) = resolved.get("audio_in") {
                    let block = crate::modules::builtins::AudioOutput::mixdown(voices, frames);
                    for (o, s) in mix.iter_mut().zip(block.iter()) {
                        o[0] += s[0];
                        o[1] += s[1];
                    }
                }
            }

            for (port, block) in outputs {
                published.insert(
                    Endpoint {
                        module: module_id,
                        port,
                    },
                    block,
                );
            }
        }

        for s in &mut mix {
            s[0] = s[0].clamp(-1.0, 1.0);
            s[1] = s[1].clamp(-1.0, 1.0);
        }
        self.sample_pos += frames as u64;
        mix
    }

    /// The normative input pipeline (PRD §6.8): collect wires in saved order,
    /// merge per signal type, apply the shared input control, clamp, deliver.
    fn resolve_input(
        &mut self,
        module_doc: &crate::document::ModuleInstance,
        port: &str,
        published: &HashMap<Endpoint, SignalBlock>,
        sample_rate: u32,
        frames: usize,
    ) -> SignalBlock {
        let endpoint = Endpoint {
            module: module_doc.id,
            port: port.to_string(),
        };
        let rep = self
            .graph
            .sync_representative
            .get(&endpoint)
            .cloned()
            .unwrap_or_else(|| endpoint.clone());
        let empty = Vec::new();
        let sources = self.graph.input_sources.get(&rep).unwrap_or(&empty);
        let blocks: Vec<&SignalBlock> = sources
            .iter()
            .filter_map(|w| published.get(&w.source))
            .collect();

        let input_state = module_doc.inputs.get(port).cloned().unwrap_or_else(|| {
            let signal = self.specs[&module_doc.id]
                .inputs
                .iter()
                .find(|p| p.id == port)
                .map(|p| p.signal)
                .unwrap_or(SignalType::Control);
            InputState::default_for(signal)
        });

        match input_state {
            InputState::Clock { manual_hz } => {
                // At most one source (validated at build). Connected inputs
                // follow the incoming phase exactly.
                if let Some(SignalBlock::Clock(Some(lane))) = blocks.first() {
                    SignalBlock::Clock(Some((*lane).clone()))
                } else {
                    // Manual frequency: phase from the deterministic origin.
                    let mut lane = Vec::with_capacity(frames);
                    for f in 0..frames {
                        let t = (self.sample_pos + f as u64) as f64 / sample_rate as f64;
                        lane.push(((t * manual_hz * TAU as f64) % TAU as f64) as f32);
                    }
                    SignalBlock::Clock(Some(lane))
                }
            }
            InputState::Note {
                manual_hz,
                transpose_semitones,
            } => {
                let (merged, _overflow) = merge::merge_note(&blocks);
                SignalBlock::Note(merge::apply_note_input(
                    merged,
                    manual_hz as f32,
                    transpose_semitones as f32,
                    frames,
                ))
            }
            InputState::Audio { gain, .. } => {
                let (mut merged, _overflow) = merge::merge_audio(&blocks);
                if merged.is_empty() {
                    merged = self.default_audio(&endpoint, frames, sample_rate);
                }
                SignalBlock::Audio(merge::apply_audio_gain(merged, gain as f32))
            }
            InputState::Control { baseline, window } => {
                let merged = merge::merge_control(&blocks, frames);
                SignalBlock::Control(merge::apply_control_input(
                    merged,
                    baseline as f32,
                    window as f32,
                    frames,
                ))
            }
            InputState::Gate { latched } => {
                let merged = merge::merge_gate(&blocks, frames);
                SignalBlock::Gate(merge::apply_gate_input(merged, latched, frames))
            }
        }
    }

    /// A disconnected Audio input produces its selected default source
    /// (PRD §6.4) with a deterministic per-input seed.
    fn default_audio(
        &mut self,
        endpoint: &Endpoint,
        frames: usize,
        sample_rate: u32,
    ) -> Vec<Voice> {
        let Some(state) = self.default_sources.get_mut(endpoint) else {
            return Vec::new();
        };
        match state.kind {
            AudioDefaultSource::Silence => Vec::new(),
            AudioDefaultSource::WhiteNoise => {
                let mut lane = Vec::with_capacity(frames);
                for _ in 0..frames {
                    let mut x = state.noise_state;
                    x ^= x >> 12;
                    x ^= x << 25;
                    x ^= x >> 27;
                    state.noise_state = x;
                    let v = x.wrapping_mul(0x2545F4914F6CDD1D);
                    lane.push(((v >> 40) as f32 / 8_388_608.0) - 1.0);
                }
                vec![Voice::mono(lane)]
            }
            AudioDefaultSource::Sine440
            | AudioDefaultSource::Saw440
            | AudioDefaultSource::Triangle440
            | AudioDefaultSource::Square440 => {
                let mut lane = Vec::with_capacity(frames);
                for _ in 0..frames {
                    let t = state.phase / TAU;
                    let s = match state.kind {
                        AudioDefaultSource::Sine440 => state.phase.sin(),
                        AudioDefaultSource::Saw440 => 2.0 * t - 1.0,
                        AudioDefaultSource::Triangle440 => {
                            if t < 0.5 {
                                4.0 * t - 1.0
                            } else {
                                3.0 - 4.0 * t
                            }
                        }
                        AudioDefaultSource::Square440 => {
                            if t < 0.5 {
                                1.0
                            } else {
                                -1.0
                            }
                        }
                        AudioDefaultSource::Silence | AudioDefaultSource::WhiteNoise => {
                            unreachable!()
                        }
                    };
                    lane.push(s);
                    state.phase = (state.phase + TAU * 440.0 / sample_rate as f32) % TAU;
                }
                vec![Voice::mono(lane)]
            }
        }
    }
}

/// Render `seconds` of audio from a Rack document to interleaved stereo f32.
pub fn render_offline(doc: RackDocument, seconds: f64) -> Result<Vec<[f32; 2]>, EngineError> {
    let sample_rate = doc.engine.sample_rate;
    let block = doc.engine.block_size as usize;
    let mut engine = Engine::new(doc)?;
    let total = (seconds * sample_rate as f64).round() as usize;
    let mut out = Vec::with_capacity(total);
    let mut rendered = 0usize;
    while rendered < total {
        let frames = block.min(total - rendered);
        out.extend(engine.process_block(sample_rate, frames));
        rendered += frames;
    }
    Ok(out)
}

/// Render a Rack document to a 16-bit stereo WAV file.
pub fn render_to_wav(
    doc: RackDocument,
    seconds: f64,
    path: &std::path::Path,
) -> Result<(), EngineError> {
    let sample_rate = doc.engine.sample_rate;
    let samples = render_offline(doc, seconds)?;
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec)?;
    for s in samples {
        writer.write_sample((s[0] * i16::MAX as f32) as i16)?;
        writer.write_sample((s[1] * i16::MAX as f32) as i16)?;
    }
    writer.finalize()?;
    Ok(())
}
