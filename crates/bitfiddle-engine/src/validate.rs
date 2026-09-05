//! Rack document validation: JSON Schema (draft 2020-12) plus the semantic
//! constraints JSON Schema cannot express (PRD §9.6).

use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use crate::document::RackDocument;
use crate::graph::build_graph;
use crate::modules::registry::{builtin_spec, ModuleSpec};

pub const RACK_SCHEMA: &str = include_str!("../../../schemas/rack.schema.json");

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("schema violation: {0}")]
    Schema(String),
    #[error("duplicate module id: {0}")]
    DuplicateModuleId(Uuid),
    #[error("duplicate module name: {0}")]
    DuplicateModuleName(String),
    #[error("duplicate wire id: {0}")]
    DuplicateWireId(Uuid),
    #[error("selected module does not exist: {0}")]
    UnknownSelection(Uuid),
    #[error("modules overlap: {0} and {1}")]
    Overlap(String, String),
    #[error("input state signal mismatch on {module}.{port}")]
    InputSignalMismatch { module: String, port: String },
    #[error("sync group members have divergent saved input state: {0}")]
    DivergentSyncState(String),
    #[error(transparent)]
    Graph(#[from] crate::graph::GraphError),
    #[error("unknown module type: {0}")]
    UnknownModuleType(String),
}

/// Validate the parsed document against the normative JSON Schema.
pub fn validate_schema(doc_yaml: &str) -> Result<(), ValidationError> {
    let value: serde_json::Value =
        serde_yaml::from_str(doc_yaml).map_err(|e| ValidationError::Schema(e.to_string()))?;
    let schema: serde_json::Value =
        serde_json::from_str(RACK_SCHEMA).expect("bundled schema parses");
    let compiled = jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .compile(&schema)
        .expect("bundled schema compiles");
    if let Err(errors) = compiled.validate(&value) {
        let msgs: Vec<String> = errors
            .map(|e| format!("{} at {}", e, e.instance_path))
            .collect();
        return Err(ValidationError::Schema(msgs.join("; ")));
    }
    Ok(())
}

/// Semantic validation beyond the schema (PRD §9.6).
pub fn validate_semantics(doc: &RackDocument) -> Result<(), ValidationError> {
    // Unique identities.
    let mut ids = HashSet::new();
    let mut names = HashSet::new();
    for m in &doc.modules {
        if !ids.insert(m.id) {
            return Err(ValidationError::DuplicateModuleId(m.id));
        }
        if !names.insert(m.name.clone()) {
            return Err(ValidationError::DuplicateModuleName(m.name.clone()));
        }
    }
    let mut wire_ids = HashSet::new();
    for w in &doc.wires {
        if !wire_ids.insert(w.id) {
            return Err(ValidationError::DuplicateWireId(w.id));
        }
    }
    for s in &doc.input_sync {
        if !wire_ids.insert(s.id) {
            return Err(ValidationError::DuplicateWireId(s.id));
        }
    }

    // view.selected refers only to current modules.
    for sel in &doc.view.selected {
        if !ids.contains(sel) {
            return Err(ValidationError::UnknownSelection(*sel));
        }
    }

    // Resolve specs; overlap check uses module dimensions.
    let mut specs: HashMap<Uuid, ModuleSpec> = HashMap::new();
    for m in &doc.modules {
        let spec = builtin_spec(&m.type_id)
            .ok_or_else(|| ValidationError::UnknownModuleType(m.type_id.clone()))?;
        specs.insert(m.id, spec);
    }

    for (i, a) in doc.modules.iter().enumerate() {
        let sa = &specs[&a.id];
        for b in doc.modules.iter().skip(i + 1) {
            let sb = &specs[&b.id];
            let ax0 = a.position.x;
            let ay0 = a.position.y;
            let ax1 = ax0 + sa.width_units as i64;
            let ay1 = ay0 + sa.height_units as i64;
            let bx0 = b.position.x;
            let by0 = b.position.y;
            let bx1 = bx0 + sb.width_units as i64;
            let by1 = by0 + sb.height_units as i64;
            if ax0 < bx1 && bx0 < ax1 && ay0 < by1 && by0 < ay1 {
                return Err(ValidationError::Overlap(a.name.clone(), b.name.clone()));
            }
        }
    }

    // Saved input state matches the declared port signal type.
    for m in &doc.modules {
        let spec = &specs[&m.id];
        for (port, state) in &m.inputs {
            let decl = spec.inputs.iter().find(|p| &p.id == port);
            match decl {
                Some(p) if p.signal == state.signal_type() => {}
                _ => {
                    return Err(ValidationError::InputSignalMismatch {
                        module: m.name.clone(),
                        port: port.clone(),
                    })
                }
            }
        }
    }

    // Graph validation: endpoints, signal match, clock sources, cycles.
    let graph = build_graph(doc, &specs)?;

    // Sync group members must hold identical signal-specific saved state.
    for (rep, members) in &graph.sync_members {
        if members.len() < 2 {
            continue;
        }
        let states: Vec<_> = members
            .iter()
            .filter_map(|e| {
                doc.modules
                    .iter()
                    .find(|m| m.id == e.module)
                    .and_then(|m| m.inputs.get(&e.port))
            })
            .collect();
        if states.len() > 1 && !states.windows(2).all(|w| w[0] == w[1]) {
            return Err(ValidationError::DivergentSyncState(format!(
                "{}.{}",
                rep.module, rep.port
            )));
        }
    }

    Ok(())
}

/// Full validation used by Apply: schema then semantics.
pub fn validate_document(yaml_text: &str) -> Result<RackDocument, ValidationError> {
    validate_schema(yaml_text)?;
    let doc =
        RackDocument::from_yaml(yaml_text).map_err(|e| ValidationError::Schema(e.to_string()))?;
    validate_semantics(&doc)?;
    Ok(doc)
}
