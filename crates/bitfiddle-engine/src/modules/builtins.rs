//! Built-in modules from the initial module set (PRD §14).
//!
//! Each built-in has a `ModuleSpec` (ports, size) and a `DspModule`
//! implementation used identically for live and offline rendering.

use std::collections::HashMap;
use std::f32::consts::TAU;

use serde_json::Value;

use crate::modules::registry::{input, output, ModuleSpec};
use crate::modules::{DspModule, ProcessCtx};
use crate::signal::{SignalBlock, SignalType, Voice};

pub fn spec(type_id: &str) -> Option<ModuleSpec> {
    let s = match type_id {
        "app.oscillator" => ModuleSpec {
            type_id: type_id.into(),
            name: "Oscillator".into(),
            version: "2.0.0".into(),
            description: "Polyphonic waveform oscillator with sine, saw, triangle, and square shapes.".into(),
            inputs: vec![input("note", "Note", SignalType::Note, 0)],
            outputs: vec![output("audio_out", "Audio Out", SignalType::Audio, 0)],
            latency_samples: 0,
            width_units: 4,
            height_units: 4,
        },
        "app.volume" => ModuleSpec {
            type_id: type_id.into(),
            name: "Volume".into(),
            version: "2.0.0".into(),
            description: "Polyphonic volume with a Control level input and live level display.".into(),
            inputs: vec![
                input("audio_in", "Audio In", SignalType::Audio, 0),
                input("level", "Level", SignalType::Control, 0),
            ],
            outputs: vec![output("audio_out", "Audio Out", SignalType::Audio, 0)],
            latency_samples: 0,
            width_units: 4,
            height_units: 4,
        },
        "app.adsr" => ModuleSpec {
            type_id: type_id.into(),
            name: "ADSR".into(),
            version: "2.0.0".into(),
            description: "Attack/decay/sustain/release envelope generator driven by a Gate input.".into(),
            inputs: vec![input("gate", "Gate", SignalType::Gate, 0)],
            outputs: vec![output("envelope", "Envelope", SignalType::Control, 0)],
            latency_samples: 0,
            width_units: 4,
            height_units: 4,
        },
        "app.clock" => ModuleSpec {
            type_id: type_id.into(),
            name: "Clock".into(),
            version: "2.0.0".into(),
            description: "Clock source globally phase-aligned with every Clock module.".into(),
            inputs: vec![input("rate", "Rate", SignalType::Clock, 0)],
            outputs: vec![output("clock_out", "Clock Out", SignalType::Clock, 0)],
            latency_samples: 0,
            width_units: 4,
            height_units: 4,
        },
        "app.audio_output" => ModuleSpec {
            type_id: type_id.into(),
            name: "Audio Output".into(),
            version: "2.0.0".into(),
            description: "Physical output. Mixes incoming voices to the selected device channels; clipping happens only here.".into(),
            inputs: vec![input("audio_in", "Audio In", SignalType::Audio, 0)],
            outputs: vec![],
            latency_samples: 0,
            width_units: 4,
            height_units: 4,
        },
        "app.noise" => ModuleSpec {
            type_id: type_id.into(),
            name: "Noise Generator".into(),
            version: "2.0.0".into(),
            description: "White noise generator with a deterministic saved seed.".into(),
            inputs: vec![],
            outputs: vec![output("audio_out", "Audio Out", SignalType::Audio, 0)],
            latency_samples: 0,
            width_units: 4,
            height_units: 4,
        },
        "app.qwerty" => ModuleSpec {
            type_id: type_id.into(),
            name: "QWERTY Input".into(),
            version: "2.0.0".into(),
            description: "Computer-keyboard note entry. Receives broadcast events in Keyboard (k) mode.".into(),
            inputs: vec![],
            outputs: vec![
                output("note_out", "Note Out", SignalType::Note, 0),
                output("gate_out", "Gate Out", SignalType::Gate, 0),
            ],
            latency_samples: 0,
            width_units: 8,
            height_units: 4,
        },
        "app.mixer8" => ModuleSpec {
            type_id: type_id.into(),
            name: "8-channel Mixer".into(),
            version: "2.0.0".into(),
            description: "Eight polyphonic Audio inputs with per-input gain, mute, and solo.".into(),
            inputs: (0..8)
                .map(|i| input(&format!("in_{i}"), &format!("In {}", i + 1), SignalType::Audio, i))
                .collect(),
            outputs: vec![output("audio_out", "Audio Out", SignalType::Audio, 0)],
            latency_samples: 0,
            width_units: 4,
            height_units: 12,
        },
        "app.scope" => ModuleSpec {
            type_id: type_id.into(),
            name: "Oscilloscope".into(),
            version: "2.0.0".into(),
            description: "Waveform display with trigger and time-window state.".into(),
            inputs: vec![input("audio_in", "Audio In", SignalType::Audio, 0)],
            outputs: vec![],
            latency_samples: 0,
            width_units: 8,
            height_units: 4,
        },
        "app.eq" => ModuleSpec {
            type_id: type_id.into(),
            name: "EQ".into(),
            version: "2.0.0".into(),
            description: "Three-band equalizer: low shelf, peaking mid, high shelf.".into(),
            inputs: vec![input("audio_in", "Audio In", SignalType::Audio, 0)],
            outputs: vec![output("audio_out", "Audio Out", SignalType::Audio, 0)],
            latency_samples: 0,
            width_units: 8,
            height_units: 4,
        },
        "app.audio_file" => ModuleSpec {
            type_id: type_id.into(),
            name: "Audio File Generator".into(),
            version: "2.0.0".into(),
            description: "Plays an audio file with one-shot, retrigger, and loop behavior driven by a Gate input.".into(),
            inputs: vec![input("gate", "Gate", SignalType::Gate, 0)],
            outputs: vec![output("audio_out", "Audio Out", SignalType::Audio, 0)],
            latency_samples: 0,
            width_units: 8,
            height_units: 4,
        },
        _ => return None,
    };
    Some(s)
}

pub fn all_type_ids() -> &'static [&'static str] {
    &[
        "app.oscillator",
        "app.volume",
        "app.adsr",
        "app.clock",
        "app.audio_output",
        "app.noise",
        "app.qwerty",
        "app.mixer8",
        "app.scope",
        "app.eq",
        "app.audio_file",
    ]
}

pub fn instantiate(
    type_id: &str,
    parameters: &serde_json::Map<String, Value>,
) -> Option<Box<dyn DspModule>> {
    let m: Box<dyn DspModule> = match type_id {
        "app.oscillator" => Box::new(Oscillator::new(parameters)),
        "app.volume" => Box::new(Volume),
        "app.adsr" => Box::new(Adsr::new(parameters)),
        "app.clock" => Box::new(ClockModule),
        "app.audio_output" => Box::new(AudioOutput::default()),
        "app.noise" => Box::new(Noise::new(parameters)),
        "app.qwerty" => Box::new(Qwerty),
        "app.mixer8" => Box::new(Mixer8::new(parameters)),
        "app.scope" => Box::new(Scope),
        "app.eq" => Box::new(Eq::new(parameters)),
        "app.audio_file" => Box::new(AudioFile::new(parameters)),
        _ => return None,
    };
    Some(m)
}

fn param_f32(p: &serde_json::Map<String, Value>, key: &str, default: f32) -> f32 {
    p.get(key)
        .and_then(Value::as_f64)
        .map(|v| v as f32)
        .unwrap_or(default)
}

fn param_str<'a>(p: &'a serde_json::Map<String, Value>, key: &str, default: &'a str) -> &'a str {
    p.get(key).and_then(Value::as_str).unwrap_or(default)
}

fn param_u64(p: &serde_json::Map<String, Value>, key: &str, default: u64) -> u64 {
    p.get(key).and_then(Value::as_u64).unwrap_or(default)
}

// ---------------------------------------------------------------------------
// Oscillator

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Waveform {
    Sine,
    Saw,
    Triangle,
    Square,
}

pub struct Oscillator {
    waveform: Waveform,
    phases: Vec<f32>,
}

impl Oscillator {
    pub fn new(params: &serde_json::Map<String, Value>) -> Self {
        let waveform = match param_str(params, "waveform", "sine") {
            "saw" => Waveform::Saw,
            "triangle" => Waveform::Triangle,
            "square" => Waveform::Square,
            _ => Waveform::Sine,
        };
        Oscillator {
            waveform,
            phases: vec![0.0; crate::MAX_CHANNELS],
        }
    }

    fn sample(waveform: Waveform, phase: f32) -> f32 {
        let t = phase / TAU; // [0, 1)
        match waveform {
            Waveform::Sine => phase.sin(),
            Waveform::Saw => 2.0 * t - 1.0,
            Waveform::Triangle => {
                if t < 0.5 {
                    4.0 * t - 1.0
                } else {
                    3.0 - 4.0 * t
                }
            }
            Waveform::Square => {
                if t < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
        }
    }
}

impl DspModule for Oscillator {
    fn process(
        &mut self,
        ctx: &ProcessCtx,
        inputs: &HashMap<String, SignalBlock>,
    ) -> HashMap<String, SignalBlock> {
        let empty: Vec<Vec<f32>> = Vec::new();
        let notes = match inputs.get("note") {
            Some(SignalBlock::Note(chans)) => chans,
            _ => &empty,
        };
        let mut voices = Vec::with_capacity(notes.len());
        for (vi, lane) in notes.iter().enumerate() {
            let phase = &mut self.phases[vi];
            let mut left = Vec::with_capacity(ctx.frames);
            for f in 0..ctx.frames {
                let hz = lane.get(f).copied().unwrap_or(440.0);
                left.push(Self::sample(self.waveform, *phase));
                *phase = (*phase + TAU * hz / ctx.sample_rate as f32) % TAU;
            }
            voices.push(Voice::mono(left));
        }
        HashMap::from([("audio_out".to_string(), SignalBlock::Audio(voices))])
    }

    fn reset(&mut self) {
        self.phases.iter_mut().for_each(|p| *p = 0.0);
    }
}

// ---------------------------------------------------------------------------
// Volume

pub struct Volume;

impl DspModule for Volume {
    fn process(
        &mut self,
        ctx: &ProcessCtx,
        inputs: &HashMap<String, SignalBlock>,
    ) -> HashMap<String, SignalBlock> {
        let voices = match inputs.get("audio_in") {
            Some(SignalBlock::Audio(v)) => v.clone(),
            _ => Vec::new(),
        };
        let level: Vec<Vec<f32>> = match inputs.get("level") {
            Some(SignalBlock::Control(c)) => c.clone(),
            _ => Vec::new(),
        };
        let mut out = voices;
        for v in &mut out {
            for (f, s) in v.left.iter_mut().enumerate() {
                let l = level
                    .first()
                    .and_then(|lane| lane.get(f))
                    .copied()
                    .unwrap_or(1.0);
                // Control is [-1, 1]; map to [0, 1] gain around unipolar use.
                *s *= l.max(0.0);
            }
            if let Some(r) = &mut v.right {
                for (f, s) in r.iter_mut().enumerate() {
                    let l = level
                        .first()
                        .and_then(|lane| lane.get(f))
                        .copied()
                        .unwrap_or(1.0);
                    *s *= l.max(0.0);
                }
            }
        }
        let _ = ctx;
        HashMap::from([("audio_out".to_string(), SignalBlock::Audio(out))])
    }
}

// ---------------------------------------------------------------------------
// ADSR

pub struct Adsr {
    attack_s: f32,
    decay_s: f32,
    sustain: f32,
    release_s: f32,
    // Per-channel envelope state.
    level: Vec<f32>,
    stage: Vec<Stage>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

impl Adsr {
    pub fn new(params: &serde_json::Map<String, Value>) -> Self {
        Adsr {
            attack_s: param_f32(params, "attack", 0.01),
            decay_s: param_f32(params, "decay", 0.1),
            sustain: param_f32(params, "sustain", 0.7),
            release_s: param_f32(params, "release", 0.2),
            level: vec![0.0; crate::MAX_CHANNELS],
            stage: vec![Stage::Idle; crate::MAX_CHANNELS],
        }
    }
}

impl DspModule for Adsr {
    fn process(
        &mut self,
        ctx: &ProcessCtx,
        inputs: &HashMap<String, SignalBlock>,
    ) -> HashMap<String, SignalBlock> {
        let empty: Vec<Vec<bool>> = Vec::new();
        let gates = match inputs.get("gate") {
            Some(SignalBlock::Gate(g)) => g,
            _ => &empty,
        };
        let chans = gates.len().clamp(1, crate::MAX_CHANNELS);
        let sr = ctx.sample_rate as f32;
        let attack_step = if self.attack_s > 0.0 {
            1.0 / (self.attack_s * sr)
        } else {
            1.0
        };
        let decay_step = if self.decay_s > 0.0 {
            1.0 / (self.decay_s * sr)
        } else {
            1.0
        };
        let release_step = if self.release_s > 0.0 {
            1.0 / (self.release_s * sr)
        } else {
            1.0
        };

        let mut out = Vec::with_capacity(chans);
        for ci in 0..chans {
            let mut lane = Vec::with_capacity(ctx.frames);
            for f in 0..ctx.frames {
                let gate_on = gates
                    .get(ci)
                    .and_then(|l| l.get(f))
                    .copied()
                    .unwrap_or(false);
                let stage = &mut self.stage[ci];
                let level = &mut self.level[ci];
                if gate_on {
                    if matches!(*stage, Stage::Idle | Stage::Release) {
                        *stage = Stage::Attack;
                    }
                } else if !matches!(*stage, Stage::Idle) {
                    *stage = Stage::Release;
                }
                match *stage {
                    Stage::Attack => {
                        *level += attack_step;
                        if *level >= 1.0 {
                            *level = 1.0;
                            *stage = Stage::Decay;
                        }
                    }
                    Stage::Decay => {
                        *level -= decay_step;
                        if *level <= self.sustain {
                            *level = self.sustain;
                            *stage = Stage::Sustain;
                        }
                    }
                    Stage::Sustain => {}
                    Stage::Release => {
                        *level -= release_step;
                        if *level <= 0.0 {
                            *level = 0.0;
                            *stage = Stage::Idle;
                        }
                    }
                    Stage::Idle => {}
                }
                lane.push(*level);
            }
            out.push(lane);
        }
        HashMap::from([("envelope".to_string(), SignalBlock::Control(out))])
    }

    fn reset(&mut self) {
        self.level.iter_mut().for_each(|l| *l = 0.0);
        self.stage.iter_mut().for_each(|s| *s = Stage::Idle);
    }
}

// ---------------------------------------------------------------------------
// Clock

/// Phase origin derives from one deterministic origin shared by every Clock
/// module (offline: sample 0). It is runtime-derived, never persisted.
#[derive(Default)]
pub struct ClockModule;

impl DspModule for ClockModule {
    fn process(
        &mut self,
        ctx: &ProcessCtx,
        inputs: &HashMap<String, SignalBlock>,
    ) -> HashMap<String, SignalBlock> {
        // The rate input is a Clock input: when connected, follow it exactly;
        // when disconnected, the merged input already carries manual phase.
        let out = match inputs.get("rate") {
            Some(SignalBlock::Clock(Some(lane))) => Some(lane.clone()),
            _ => None,
        };
        let _ = ctx;
        HashMap::from([("clock_out".to_string(), SignalBlock::Clock(out))])
    }
}

// ---------------------------------------------------------------------------
// Audio Output

/// Terminal sink. Captures the final mixed stereo block; clipping to
/// [-1, 1] happens only at this physical boundary (PRD §5.1).
#[derive(Default)]
pub struct AudioOutput {
    pub last_block: Vec<[f32; 2]>,
}

impl AudioOutput {
    /// Mix incoming voices down to stereo and clip.
    pub fn mixdown(voices: &[Voice], frames: usize) -> Vec<[f32; 2]> {
        let mut out = vec![[0.0f32; 2]; frames];
        for v in voices {
            for (f, s) in out.iter_mut().enumerate() {
                s[0] += v.left.get(f).copied().unwrap_or(0.0);
                s[1] += v.right_lane().get(f).copied().unwrap_or(0.0);
            }
        }
        for s in &mut out {
            s[0] = s[0].clamp(-1.0, 1.0);
            s[1] = s[1].clamp(-1.0, 1.0);
        }
        out
    }
}

impl DspModule for AudioOutput {
    fn process(
        &mut self,
        ctx: &ProcessCtx,
        inputs: &HashMap<String, SignalBlock>,
    ) -> HashMap<String, SignalBlock> {
        let voices = match inputs.get("audio_in") {
            Some(SignalBlock::Audio(v)) => v.as_slice(),
            _ => &[],
        };
        self.last_block = Self::mixdown(voices, ctx.frames);
        HashMap::new()
    }
}

// ---------------------------------------------------------------------------
// Noise

/// Deterministic white noise via xorshift64*, seeded from Rack state.
pub struct Noise {
    state: u64,
    seed: u64,
}

impl Noise {
    pub fn new(params: &serde_json::Map<String, Value>) -> Self {
        let seed = param_u64(params, "seed", 1).max(1);
        Noise { state: seed, seed }
    }

    fn next(&mut self) -> f32 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        let v = x.wrapping_mul(0x2545F4914F6CDD1D);
        // Map the top 24 bits to [-1, 1).
        ((v >> 40) as f32 / 8_388_608.0) - 1.0
    }
}

impl DspModule for Noise {
    fn process(
        &mut self,
        ctx: &ProcessCtx,
        _inputs: &HashMap<String, SignalBlock>,
    ) -> HashMap<String, SignalBlock> {
        let mut left = Vec::with_capacity(ctx.frames);
        for _ in 0..ctx.frames {
            left.push(self.next());
        }
        HashMap::from([(
            "audio_out".to_string(),
            SignalBlock::Audio(vec![Voice::mono(left)]),
        )])
    }

    fn reset(&mut self) {
        self.state = self.seed;
    }
}

// ---------------------------------------------------------------------------
// QWERTY Input

/// Headless QWERTY produces no notes; live key events are delivered by the
/// host in Keyboard (k) mode. Offline rendering treats it as silent.
pub struct Qwerty;

impl DspModule for Qwerty {
    fn process(
        &mut self,
        _ctx: &ProcessCtx,
        _inputs: &HashMap<String, SignalBlock>,
    ) -> HashMap<String, SignalBlock> {
        HashMap::from([
            ("note_out".to_string(), SignalBlock::Note(Vec::new())),
            ("gate_out".to_string(), SignalBlock::Gate(Vec::new())),
        ])
    }
}

// ---------------------------------------------------------------------------
// 8-channel Mixer

pub struct Mixer8 {
    gains: [f32; 8],
    mutes: [bool; 8],
    solos: [bool; 8],
}

impl Mixer8 {
    pub fn new(params: &serde_json::Map<String, Value>) -> Self {
        let mut gains = [1.0f32; 8];
        let mut mutes = [false; 8];
        let mut solos = [false; 8];
        for i in 0..8 {
            gains[i] = param_f32(params, &format!("gain_{i}"), 1.0);
            mutes[i] = params
                .get(&format!("mute_{i}"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            solos[i] = params
                .get(&format!("solo_{i}"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
        }
        Mixer8 {
            gains,
            mutes,
            solos,
        }
    }
}

impl DspModule for Mixer8 {
    fn process(
        &mut self,
        _ctx: &ProcessCtx,
        inputs: &HashMap<String, SignalBlock>,
    ) -> HashMap<String, SignalBlock> {
        let any_solo = self.solos.iter().any(|s| *s);
        let mut out: Vec<Voice> = Vec::new();
        for i in 0..8 {
            if self.mutes[i] || (any_solo && !self.solos[i]) {
                continue;
            }
            if let Some(SignalBlock::Audio(voices)) = inputs.get(&format!("in_{i}")) {
                for v in voices {
                    if out.len() >= crate::MAX_CHANNELS {
                        break;
                    }
                    let mut nv = v.clone();
                    for s in &mut nv.left {
                        *s *= self.gains[i];
                    }
                    if let Some(r) = &mut nv.right {
                        for s in r {
                            *s *= self.gains[i];
                        }
                    }
                    out.push(nv);
                }
            }
        }
        HashMap::from([("audio_out".to_string(), SignalBlock::Audio(out))])
    }
}

// ---------------------------------------------------------------------------
// Oscilloscope

/// Display-only sink; the UI reads bounded capture through host telemetry.
pub struct Scope;

impl DspModule for Scope {
    fn process(
        &mut self,
        _ctx: &ProcessCtx,
        _inputs: &HashMap<String, SignalBlock>,
    ) -> HashMap<String, SignalBlock> {
        HashMap::new()
    }
}

// ---------------------------------------------------------------------------
// EQ

/// One biquad section (RBJ cookbook) with per-lane state.
#[derive(Clone)]
struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    // Direct form II transposed state per lane (32 audio lanes).
    z1: Vec<f32>,
    z2: Vec<f32>,
}

impl Biquad {
    fn identity() -> Self {
        Biquad {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            z1: vec![0.0; crate::MAX_AUDIO_LANES],
            z2: vec![0.0; crate::MAX_AUDIO_LANES],
        }
    }

    fn low_shelf(sample_rate: f32, freq: f32, gain_db: f32) -> Self {
        Self::shelf(sample_rate, freq, gain_db, true)
    }

    fn high_shelf(sample_rate: f32, freq: f32, gain_db: f32) -> Self {
        Self::shelf(sample_rate, freq, gain_db, false)
    }

    fn shelf(sample_rate: f32, freq: f32, gain_db: f32, low: bool) -> Self {
        let a = 10f32.powf(gain_db / 40.0);
        let w0 = TAU * freq / sample_rate;
        let (sin, cos) = w0.sin_cos();
        let alpha = sin / 2.0 * (2.0f32).sqrt();
        let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;
        let (b0, b1, b2, a0, a1, a2) = if low {
            (
                a * ((a + 1.0) - (a - 1.0) * cos + two_sqrt_a_alpha),
                2.0 * a * ((a - 1.0) - (a + 1.0) * cos),
                a * ((a + 1.0) - (a - 1.0) * cos - two_sqrt_a_alpha),
                (a + 1.0) + (a - 1.0) * cos + two_sqrt_a_alpha,
                -2.0 * ((a - 1.0) + (a + 1.0) * cos),
                (a + 1.0) + (a - 1.0) * cos - two_sqrt_a_alpha,
            )
        } else {
            (
                a * ((a + 1.0) + (a - 1.0) * cos + two_sqrt_a_alpha),
                -2.0 * a * ((a - 1.0) + (a + 1.0) * cos),
                a * ((a + 1.0) + (a - 1.0) * cos - two_sqrt_a_alpha),
                (a + 1.0) - (a - 1.0) * cos + two_sqrt_a_alpha,
                2.0 * ((a - 1.0) - (a + 1.0) * cos),
                (a + 1.0) - (a - 1.0) * cos - two_sqrt_a_alpha,
            )
        };
        Biquad {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
            z1: vec![0.0; crate::MAX_AUDIO_LANES],
            z2: vec![0.0; crate::MAX_AUDIO_LANES],
        }
    }

    fn peaking(sample_rate: f32, freq: f32, gain_db: f32, q: f32) -> Self {
        let a = 10f32.powf(gain_db / 40.0);
        let w0 = TAU * freq / sample_rate;
        let (sin, cos) = w0.sin_cos();
        let alpha = sin / (2.0 * q);
        let a0 = 1.0 + alpha / a;
        Biquad {
            b0: (1.0 + alpha * a) / a0,
            b1: (-2.0 * cos) / a0,
            b2: (1.0 - alpha * a) / a0,
            a1: (-2.0 * cos) / a0,
            a2: (1.0 - alpha / a) / a0,
            z1: vec![0.0; crate::MAX_AUDIO_LANES],
            z2: vec![0.0; crate::MAX_AUDIO_LANES],
        }
    }

    fn tick(&mut self, lane: usize, x: f32) -> f32 {
        let y = self.b0 * x + self.z1[lane];
        self.z1[lane] = self.b1 * x - self.a1 * y + self.z2[lane];
        self.z2[lane] = self.b2 * x - self.a2 * y;
        y
    }

    fn reset(&mut self) {
        self.z1.iter_mut().for_each(|z| *z = 0.0);
        self.z2.iter_mut().for_each(|z| *z = 0.0);
    }
}

/// Three-band EQ: low shelf (200 Hz), peaking mid (1 kHz), high shelf (4 kHz).
pub struct Eq {
    low_db: f32,
    mid_db: f32,
    high_db: f32,
    sections: Option<[Biquad; 3]>,
    configured_rate: u32,
}

impl Eq {
    pub fn new(params: &serde_json::Map<String, Value>) -> Self {
        Eq {
            low_db: param_f32(params, "low_db", 0.0),
            mid_db: param_f32(params, "mid_db", 0.0),
            high_db: param_f32(params, "high_db", 0.0),
            sections: None,
            configured_rate: 0,
        }
    }

    fn ensure_sections(&mut self, sample_rate: u32) {
        if self.sections.is_none() || self.configured_rate != sample_rate {
            let sr = sample_rate as f32;
            let flat = self.low_db == 0.0 && self.mid_db == 0.0 && self.high_db == 0.0;
            self.sections = Some(if flat {
                [Biquad::identity(), Biquad::identity(), Biquad::identity()]
            } else {
                [
                    Biquad::low_shelf(sr, 200.0, self.low_db),
                    Biquad::peaking(sr, 1_000.0, self.mid_db, 0.9),
                    Biquad::high_shelf(sr, 4_000.0, self.high_db),
                ]
            });
            self.configured_rate = sample_rate;
        }
    }
}

impl DspModule for Eq {
    fn process(
        &mut self,
        ctx: &ProcessCtx,
        inputs: &HashMap<String, SignalBlock>,
    ) -> HashMap<String, SignalBlock> {
        self.ensure_sections(ctx.sample_rate);
        let voices = match inputs.get("audio_in") {
            Some(SignalBlock::Audio(v)) => v.clone(),
            _ => Vec::new(),
        };
        let sections = self.sections.as_mut().unwrap();
        let mut out = voices;
        for (vi, v) in out.iter_mut().enumerate() {
            let left_lane = vi * 2;
            for s in v.left.iter_mut() {
                for sec in sections.iter_mut() {
                    *s = sec.tick(left_lane, *s);
                }
            }
            if let Some(r) = &mut v.right {
                let right_lane = vi * 2 + 1;
                for s in r.iter_mut() {
                    for sec in sections.iter_mut() {
                        *s = sec.tick(right_lane, *s);
                    }
                }
            }
        }
        HashMap::from([("audio_out".to_string(), SignalBlock::Audio(out))])
    }

    fn reset(&mut self) {
        if let Some(sections) = &mut self.sections {
            for s in sections.iter_mut() {
                s.reset();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Audio File Generator

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlayMode {
    OneShot,
    Retrigger,
    Loop,
}

/// Plays decoded audio-file samples on gate rising edges. The decoded data is
/// loaded off-thread at construction; in the headless engine tests use the
/// injected sample buffer. A missing file produces silence, never a failure
/// to open the Rack (PRD §9.1, §14).
pub struct AudioFile {
    mode: PlayMode,
    samples: Vec<f32>,
    playing: bool,
    cursor: usize,
    prev_gate: bool,
}

impl AudioFile {
    pub fn new(params: &serde_json::Map<String, Value>) -> Self {
        let mode = match param_str(params, "mode", "one_shot") {
            "retrigger" => PlayMode::Retrigger,
            "loop" => PlayMode::Loop,
            _ => PlayMode::OneShot,
        };
        // Decoded samples are provided by the host loader; tests inject a
        // deterministic ramp when the file is absent.
        let samples = params
            .get("test_samples")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_f64)
                    .map(|v| v as f32)
                    .collect()
            })
            .unwrap_or_default();
        AudioFile {
            mode,
            samples,
            playing: false,
            cursor: 0,
            prev_gate: false,
        }
    }

    /// Host loader entry point: replace decoded sample data.
    pub fn set_samples(&mut self, samples: Vec<f32>) {
        self.samples = samples;
        self.cursor = 0;
        self.playing = false;
    }
}

impl DspModule for AudioFile {
    fn process(
        &mut self,
        ctx: &ProcessCtx,
        inputs: &HashMap<String, SignalBlock>,
    ) -> HashMap<String, SignalBlock> {
        let empty: Vec<Vec<bool>> = Vec::new();
        let gates = match inputs.get("gate") {
            Some(SignalBlock::Gate(g)) => g,
            _ => &empty,
        };
        let mut left = Vec::with_capacity(ctx.frames);
        for f in 0..ctx.frames {
            let gate = gates
                .first()
                .and_then(|l| l.get(f))
                .copied()
                .unwrap_or(false);
            let rising = gate && !self.prev_gate;
            self.prev_gate = gate;
            if rising {
                match self.mode {
                    PlayMode::OneShot => {
                        if !self.playing {
                            self.playing = true;
                            self.cursor = 0;
                        }
                    }
                    PlayMode::Retrigger | PlayMode::Loop => {
                        self.playing = true;
                        self.cursor = 0;
                    }
                }
            }
            let sample = if self.playing && !self.samples.is_empty() {
                let s = self.samples[self.cursor];
                self.cursor += 1;
                if self.cursor >= self.samples.len() {
                    if self.mode == PlayMode::Loop && gate {
                        self.cursor = 0;
                    } else {
                        self.playing = false;
                    }
                }
                s
            } else {
                0.0
            };
            left.push(sample);
        }
        let voices = if self.samples.is_empty() {
            Vec::new()
        } else {
            vec![Voice::mono(left)]
        };
        HashMap::from([("audio_out".to_string(), SignalBlock::Audio(voices))])
    }

    fn reset(&mut self) {
        self.playing = false;
        self.cursor = 0;
        self.prev_gate = false;
    }
}
