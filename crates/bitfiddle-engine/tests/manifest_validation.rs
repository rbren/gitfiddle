//! Extension manifest validation tests (PRD §11.2, §18.1).

use bitfiddle_engine::manifest::{validate_manifest, ManifestError};

fn base_manifest() -> serde_json::Value {
    serde_json::json!({
        "id": "com.example.delay",
        "name": "Delay",
        "version": "2.1.0",
        "abi": "wasm-2",
        "description": "A simple delay.",
        "deprecated": false,
        "latency_samples": 0,
        "size": { "width_units": 8, "height_units": 8 },
        "state_version": 1,
        "custom_state_schema": { "type": "object" },
        "parameters": [
            {
                "id": "feedback",
                "name": "Feedback",
                "kind": "number",
                "default": 0.25,
                "minimum": 0,
                "maximum": 0.95
            }
        ],
        "inputs": [
            {
                "id": "audio_in",
                "name": "Audio In",
                "signal": "audio",
                "description": "Input.",
                "order": 0
            }
        ],
        "outputs": [
            {
                "id": "audio_out",
                "name": "Audio Out",
                "signal": "audio",
                "description": "Output.",
                "order": 0
            }
        ],
        "flavors": [
            { "name": "Vanilla", "description": "Default.", "inputs": {}, "state": {} }
        ],
        "bypass": { "audio_out": "audio_in" },
        "presets": [],
        "ui": { "entry": "ui.js", "api": "ui-2" }
    })
}

fn text(v: &serde_json::Value) -> String {
    serde_json::to_string(v).unwrap()
}

#[test]
fn valid_manifest_passes() {
    let m = validate_manifest(&text(&base_manifest())).unwrap();
    assert_eq!(m.id, "com.example.delay");
    assert_eq!(m.inputs.len(), 1);
}

#[test]
fn unknown_field_rejected() {
    let mut v = base_manifest();
    v["unit"] = serde_json::json!("volts"); // prohibited legacy concept
    assert!(matches!(
        validate_manifest(&text(&v)),
        Err(ManifestError::Schema(_))
    ));
}

#[test]
fn dimensions_must_divide_by_four() {
    let mut v = base_manifest();
    v["size"]["width_units"] = serde_json::json!(6);
    assert!(validate_manifest(&text(&v)).is_err());
}

#[test]
fn duplicate_input_ids_rejected() {
    let mut v = base_manifest();
    let dup = v["inputs"][0].clone();
    v["inputs"].as_array_mut().unwrap().push(dup);
    assert!(matches!(
        validate_manifest(&text(&v)),
        Err(ManifestError::DuplicatePortId(_))
    ));
}

#[test]
fn invalid_bypass_route_rejected() {
    let mut v = base_manifest();
    v["bypass"] = serde_json::json!({ "audio_out": "nonexistent" });
    assert!(matches!(
        validate_manifest(&text(&v)),
        Err(ManifestError::InvalidBypass { .. })
    ));
}

#[test]
fn bypass_signal_mismatch_rejected() {
    let mut v = base_manifest();
    v["inputs"].as_array_mut().unwrap().push(serde_json::json!({
        "id": "gate_in",
        "name": "Gate",
        "signal": "gate",
        "description": "Gate.",
        "order": 1
    }));
    v["bypass"] = serde_json::json!({ "audio_out": "gate_in" });
    assert!(matches!(
        validate_manifest(&text(&v)),
        Err(ManifestError::InvalidBypass { .. })
    ));
}

#[test]
fn unsupported_signal_type_rejected() {
    let mut v = base_manifest();
    v["inputs"][0]["signal"] = serde_json::json!("voltage");
    assert!(matches!(
        validate_manifest(&text(&v)),
        Err(ManifestError::Schema(_))
    ));
}

#[test]
fn enum_parameter_default_must_be_an_option() {
    let mut v = base_manifest();
    v["parameters"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "id": "mode",
            "name": "Mode",
            "kind": "enum",
            "default": "pingpong",
            "options": ["mono", "stereo"]
        }));
    assert!(matches!(
        validate_manifest(&text(&v)),
        Err(ManifestError::InvalidParameter(_, _))
    ));
}

#[test]
fn numeric_default_outside_range_rejected() {
    let mut v = base_manifest();
    v["parameters"][0]["default"] = serde_json::json!(2.0);
    assert!(matches!(
        validate_manifest(&text(&v)),
        Err(ManifestError::InvalidParameter(_, _))
    ));
}

#[test]
fn missing_flavor_rejected() {
    let mut v = base_manifest();
    v["flavors"] = serde_json::json!([]);
    // Schema requires minItems 1, so this fails at schema level.
    assert!(validate_manifest(&text(&v)).is_err());
}

#[test]
fn port_count_limit_enforced() {
    let mut v = base_manifest();
    let inputs: Vec<serde_json::Value> = (0..129)
        .map(|i| {
            serde_json::json!({
                "id": format!("in_{i}"),
                "name": format!("In {i}"),
                "signal": "control",
                "description": "x",
                "order": i
            })
        })
        .collect();
    v["inputs"] = serde_json::json!(inputs);
    assert!(validate_manifest(&text(&v)).is_err());
}
