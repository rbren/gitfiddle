//! Rack YAML document model (PRD §9).
//!
//! The YAML document is the canonical persistent state. These types mirror
//! `schemas/rack.schema.json` exactly; the schema is validated separately in
//! `validate`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

use crate::signal::SignalType;

pub const FORMAT: &str = "bitfiddle-rack";
pub const FORMAT_VERSION: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RackDocument {
    pub format: String,
    pub format_version: u32,
    pub app_version: String,
    pub rack: RackMetadata,
    pub engine: EngineConfig,
    pub view: View,
    pub modules: Vec<ModuleInstance>,
    pub wires: Vec<Wire>,
    pub input_sync: Vec<InputSync>,
    pub macros: Vec<MacroInstance>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RackMetadata {
    pub id: Uuid,
    pub name: String,
    pub revision: u64,
    pub created_at: String,
    pub modified_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineConfig {
    pub sample_rate: u32,
    pub block_size: u32,
    pub default_device_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct View {
    pub pan: Pan,
    pub zoom: f64,
    pub selected: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pan {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GridPoint {
    pub x: i64,
    pub y: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleInstance {
    pub id: Uuid,
    pub name: String,
    pub type_id: String,
    pub type_version: String,
    pub abi: Abi,
    pub state_version: u32,
    pub flavor: String,
    pub position: GridPoint,
    pub bypassed: bool,
    #[serde(default)]
    pub input_ui: BTreeMap<String, InputUi>,
    pub inputs: BTreeMap<String, InputState>,
    pub state: ModuleState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Abi {
    #[serde(rename = "builtin-2")]
    Builtin2,
    #[serde(rename = "wasm-2")]
    Wasm2,
    #[serde(rename = "native-2")]
    Native2,
    #[serde(rename = "missing-2")]
    Missing2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputUi {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleState {
    pub parameters: serde_json::Map<String, serde_json::Value>,
    pub custom: serde_json::Map<String, serde_json::Value>,
}

/// Signal-specific saved input state (PRD §6, §9.6).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "signal", rename_all = "lowercase", deny_unknown_fields)]
pub enum InputState {
    Clock {
        manual_hz: f64,
    },
    Note {
        manual_hz: f64,
        transpose_semitones: f64,
    },
    Audio {
        gain: f64,
        default_source: AudioDefaultSource,
        seed: u64,
    },
    Control {
        baseline: f64,
        window: f64,
    },
    Gate {
        latched: bool,
    },
}

impl InputState {
    pub fn signal_type(&self) -> SignalType {
        match self {
            InputState::Clock { .. } => SignalType::Clock,
            InputState::Note { .. } => SignalType::Note,
            InputState::Audio { .. } => SignalType::Audio,
            InputState::Control { .. } => SignalType::Control,
            InputState::Gate { .. } => SignalType::Gate,
        }
    }

    pub fn default_for(signal: SignalType) -> Self {
        match signal {
            SignalType::Clock => InputState::Clock { manual_hz: 2.0 },
            SignalType::Note => InputState::Note {
                manual_hz: 440.0,
                transpose_semitones: 0.0,
            },
            SignalType::Audio => InputState::Audio {
                gain: 1.0,
                default_source: AudioDefaultSource::Silence,
                seed: 0,
            },
            SignalType::Control => InputState::Control {
                baseline: 0.0,
                window: 1.0,
            },
            SignalType::Gate => InputState::Gate { latched: false },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioDefaultSource {
    Silence,
    WhiteNoise,
    Sine440,
    Saw440,
    Triangle440,
    Square440,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Endpoint {
    pub module: Uuid,
    pub port: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Wire {
    pub id: Uuid,
    pub signal: SignalType,
    pub source: Endpoint,
    pub target: Endpoint,
    pub order: u32,
    pub waypoints: Vec<GridPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputSync {
    pub id: Uuid,
    pub signal: SignalType,
    pub a: Endpoint,
    pub b: Endpoint,
    pub waypoints: Vec<GridPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MacroInstance {
    pub module_id: Uuid,
    pub global_id: Uuid,
    pub global_name: String,
    pub format_version: u32,
    pub adopted_revision: u64,
    pub adopted_definition: serde_json::Map<String, serde_json::Value>,
    pub current_definition: Option<serde_json::Map<String, serde_json::Value>>,
}

impl RackDocument {
    /// Parse a Rack document from YAML text. Anchors/aliases are rejected by
    /// the deterministic-format rule (PRD §9.2, §17); serde_yaml resolves
    /// aliases during parse, so we reject the syntax up front.
    pub fn from_yaml(text: &str) -> Result<Self, DocumentError> {
        for line in text.lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with('#') {
                continue;
            }
            if line.contains(" &") || line.contains(" *") || trimmed.starts_with('&') {
                // A conservative pre-check would flag legitimate strings, so
                // only reject clear YAML anchor/alias tokens after a colon.
                if let Some((_, v)) = line.split_once(':') {
                    let v = v.trim();
                    if v.starts_with('&') || v.starts_with('*') {
                        return Err(DocumentError::AnchorsForbidden);
                    }
                }
            }
        }
        let doc: RackDocument = serde_yaml::from_str(text)?;
        if doc.format != FORMAT {
            return Err(DocumentError::WrongFormat(doc.format));
        }
        if doc.format_version != FORMAT_VERSION {
            return Err(DocumentError::UnsupportedVersion(doc.format_version));
        }
        Ok(doc)
    }

    /// Serialize deterministically: two-space indentation, stable key order
    /// (struct declaration order), LF endings, one terminal newline, no
    /// anchors or aliases (PRD §9.2).
    pub fn to_yaml(&self) -> Result<String, DocumentError> {
        let mut s = serde_yaml::to_string(self)?;
        if !s.ends_with('\n') {
            s.push('\n');
        }
        Ok(s)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DocumentError {
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("YAML anchors and aliases are not allowed")]
    AnchorsForbidden,
    #[error("not a bitfiddle-rack document (format: {0})")]
    WrongFormat(String),
    #[error("unsupported rack format version {0}")]
    UnsupportedVersion(u32),
}
