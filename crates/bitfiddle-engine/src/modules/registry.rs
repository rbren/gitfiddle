//! Module type registry: specs (ports, parameters, category) for built-ins
//! and, later, discovered extensions.

use serde_json::Value;

use crate::graph::PortDecl;
use crate::signal::SignalType;

/// Automatic category assignment (PRD §7.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Sequencer,
    Clock,
    Logic,
    Utility,
    Generator,
    Output,
    Effect,
    Mixer,
}

#[derive(Debug, Clone)]
pub struct ModuleSpec {
    pub type_id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub inputs: Vec<PortDecl>,
    pub outputs: Vec<PortDecl>,
    pub latency_samples: u32,
    /// width/height in grid units, multiples of four (PRD §7.6).
    pub width_units: u32,
    pub height_units: u32,
}

impl ModuleSpec {
    /// The host assigns the first matching category; manifests cannot
    /// override it (PRD §7.3).
    pub fn category(&self) -> Category {
        let has = |ports: &[PortDecl], s: SignalType| ports.iter().any(|p| p.signal == s);
        let audio_in = self
            .inputs
            .iter()
            .filter(|p| p.signal == SignalType::Audio)
            .count();
        let audio_out = self
            .outputs
            .iter()
            .filter(|p| p.signal == SignalType::Audio)
            .count();
        if audio_in == 0 && audio_out == 0 {
            if has(&self.outputs, SignalType::Note) {
                return Category::Sequencer;
            }
            if has(&self.outputs, SignalType::Clock) {
                return Category::Clock;
            }
            if has(&self.outputs, SignalType::Control) || has(&self.outputs, SignalType::Gate) {
                return Category::Logic;
            }
            return Category::Utility;
        }
        if audio_in == 0 {
            return Category::Generator;
        }
        if audio_out == 0 {
            return Category::Output;
        }
        if audio_in == 1 && audio_out == 1 {
            return Category::Effect;
        }
        if audio_in > 1 && audio_out == 1 {
            return Category::Mixer;
        }
        Category::Utility
    }
}

pub fn input(id: &str, name: &str, signal: SignalType, order: u32) -> PortDecl {
    PortDecl {
        id: id.to_string(),
        name: name.to_string(),
        signal,
        order,
        is_input: true,
    }
}

pub fn output(id: &str, name: &str, signal: SignalType, order: u32) -> PortDecl {
    PortDecl {
        id: id.to_string(),
        name: name.to_string(),
        signal,
        order,
        is_input: false,
    }
}

/// Resolve a built-in spec by type ID.
pub fn builtin_spec(type_id: &str) -> Option<ModuleSpec> {
    crate::modules::builtins::spec(type_id)
}

/// Instantiate a built-in DSP module from its saved parameters and inputs.
pub fn instantiate_builtin(
    type_id: &str,
    parameters: &serde_json::Map<String, Value>,
) -> Option<Box<dyn crate::modules::DspModule>> {
    crate::modules::builtins::instantiate(type_id, parameters)
}
