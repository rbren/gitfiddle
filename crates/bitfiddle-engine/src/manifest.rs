//! Extension manifest model and validation (PRD §11.2).
//!
//! The normative JSON Schema lives at `schemas/manifest.schema.json`;
//! semantic rules that the schema cannot express are enforced here.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::signal::SignalType;

pub const MANIFEST_SCHEMA: &str = include_str!("../../../schemas/manifest.schema.json");

pub const MAX_PORTS_PER_DIRECTION: usize = 128;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub abi: ExtensionAbi,
    pub description: String,
    pub deprecated: bool,
    pub latency_samples: u32,
    pub size: Size,
    pub state_version: u32,
    pub custom_state_schema: serde_json::Value,
    pub parameters: Vec<Parameter>,
    pub inputs: Vec<Port>,
    pub outputs: Vec<Port>,
    pub flavors: Vec<Flavor>,
    pub bypass: serde_json::Map<String, serde_json::Value>,
    pub presets: Vec<Preset>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ui: Option<UiEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExtensionAbi {
    #[serde(rename = "wasm-2")]
    Wasm2,
    #[serde(rename = "native-2")]
    Native2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Size {
    pub width_units: u32,
    pub height_units: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Parameter {
    pub id: String,
    pub name: String,
    pub kind: ParameterKind,
    pub default: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParameterKind {
    Number,
    Integer,
    Boolean,
    Enum,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Port {
    pub id: String,
    pub name: String,
    pub signal: SignalType,
    pub description: String,
    pub order: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Flavor {
    pub name: String,
    pub description: String,
    pub inputs: serde_json::Map<String, serde_json::Value>,
    pub state: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preset {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub state: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UiEntry {
    pub entry: String,
    pub api: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    #[error("manifest schema violation: {0}")]
    Schema(String),
    #[error("duplicate port id: {0}")]
    DuplicatePortId(String),
    #[error("duplicate parameter id: {0}")]
    DuplicateParameterId(String),
    #[error("dimensions must be positive multiples of four units: {0}x{1}")]
    InvalidDimensions(u32, u32),
    #[error("too many ports in one direction: {0} (max {MAX_PORTS_PER_DIRECTION})")]
    TooManyPorts(usize),
    #[error("bypass route {output} -> {input} is invalid: {reason}")]
    InvalidBypass {
        output: String,
        input: String,
        reason: String,
    },
    #[error("no flavor declared; the default flavor is Vanilla")]
    NoFlavors,
    #[error("parameter {0}: {1}")]
    InvalidParameter(String, String),
}

/// Validate manifest JSON against the normative schema, then semantics.
pub fn validate_manifest(json_text: &str) -> Result<Manifest, ManifestError> {
    let value: serde_json::Value =
        serde_json::from_str(json_text).map_err(|e| ManifestError::Schema(e.to_string()))?;
    let schema: serde_json::Value =
        serde_json::from_str(MANIFEST_SCHEMA).expect("bundled manifest schema parses");
    let compiled = jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .compile(&schema)
        .expect("bundled manifest schema compiles");
    if let Err(errors) = compiled.validate(&value) {
        let msgs: Vec<String> = errors.map(|e| e.to_string()).collect();
        return Err(ManifestError::Schema(msgs.join("; ")));
    }
    let manifest: Manifest =
        serde_json::from_value(value).map_err(|e| ManifestError::Schema(e.to_string()))?;
    validate_manifest_semantics(&manifest)?;
    Ok(manifest)
}

pub fn validate_manifest_semantics(m: &Manifest) -> Result<(), ManifestError> {
    if m.size.width_units == 0
        || m.size.height_units == 0
        || !m.size.width_units.is_multiple_of(4)
        || !m.size.height_units.is_multiple_of(4)
    {
        return Err(ManifestError::InvalidDimensions(
            m.size.width_units,
            m.size.height_units,
        ));
    }
    if m.inputs.len() > MAX_PORTS_PER_DIRECTION {
        return Err(ManifestError::TooManyPorts(m.inputs.len()));
    }
    if m.outputs.len() > MAX_PORTS_PER_DIRECTION {
        return Err(ManifestError::TooManyPorts(m.outputs.len()));
    }

    let mut input_ids = HashSet::new();
    for p in &m.inputs {
        if !input_ids.insert(p.id.as_str()) {
            return Err(ManifestError::DuplicatePortId(p.id.clone()));
        }
    }
    let mut output_ids = HashSet::new();
    for p in &m.outputs {
        if !output_ids.insert(p.id.as_str()) {
            return Err(ManifestError::DuplicatePortId(p.id.clone()));
        }
    }

    let mut param_ids = HashSet::new();
    for p in &m.parameters {
        if !param_ids.insert(p.id.as_str()) {
            return Err(ManifestError::DuplicateParameterId(p.id.clone()));
        }
        match p.kind {
            ParameterKind::Enum => {
                let options = p.options.as_deref().unwrap_or(&[]);
                if options.is_empty() {
                    return Err(ManifestError::InvalidParameter(
                        p.id.clone(),
                        "enum requires options".into(),
                    ));
                }
                let default = p.default.as_str().unwrap_or("");
                if !options.iter().any(|o| o == default) {
                    return Err(ManifestError::InvalidParameter(
                        p.id.clone(),
                        "default must be one of options".into(),
                    ));
                }
            }
            ParameterKind::Boolean => {
                if !p.default.is_boolean() {
                    return Err(ManifestError::InvalidParameter(
                        p.id.clone(),
                        "boolean default required".into(),
                    ));
                }
            }
            ParameterKind::Number | ParameterKind::Integer => {
                let Some(d) = p.default.as_f64() else {
                    return Err(ManifestError::InvalidParameter(
                        p.id.clone(),
                        "numeric default required".into(),
                    ));
                };
                if p.kind == ParameterKind::Integer && d.fract() != 0.0 {
                    return Err(ManifestError::InvalidParameter(
                        p.id.clone(),
                        "integer default must be integral".into(),
                    ));
                }
                if let (Some(min), Some(max)) = (p.minimum, p.maximum) {
                    if min > max || d < min || d > max {
                        return Err(ManifestError::InvalidParameter(
                            p.id.clone(),
                            "default outside [minimum, maximum]".into(),
                        ));
                    }
                }
            }
        }
    }

    // Bypass routes: output id -> input id, same signal type, both exist.
    let find_out = |id: &str| m.outputs.iter().find(|p| p.id == id);
    let find_in = |id: &str| m.inputs.iter().find(|p| p.id == id);
    for (out_id, in_val) in &m.bypass {
        let in_id = in_val.as_str().unwrap_or("");
        let (Some(out), Some(inp)) = (find_out(out_id), find_in(in_id)) else {
            return Err(ManifestError::InvalidBypass {
                output: out_id.clone(),
                input: in_id.to_string(),
                reason: "unknown port".into(),
            });
        };
        if out.signal != inp.signal {
            return Err(ManifestError::InvalidBypass {
                output: out_id.clone(),
                input: in_id.to_string(),
                reason: "signal type mismatch".into(),
            });
        }
    }

    if m.flavors.is_empty() {
        return Err(ManifestError::NoFlavors);
    }

    Ok(())
}
