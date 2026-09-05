//! Persistence tests: schema validation, YAML round trips, semantic
//! validation (PRD §9, §18.5).

use bitfiddle_engine::document::RackDocument;
use bitfiddle_engine::validate::{validate_document, validate_schema, ValidationError};

const SINE: &str = include_str!("../../../fixtures/sine.bitfiddle.yaml");

#[test]
fn sine_fixture_passes_schema_and_semantics() {
    validate_document(SINE).expect("fixture is valid");
}

#[test]
fn yaml_round_trip_is_stable() {
    let doc = RackDocument::from_yaml(SINE).unwrap();
    let out1 = doc.to_yaml().unwrap();
    let doc2 = RackDocument::from_yaml(&out1).unwrap();
    let out2 = doc2.to_yaml().unwrap();
    assert_eq!(out1, out2, "save -> load -> save must be byte-identical");
    assert!(out1.ends_with('\n'));
    assert!(!out1.contains('\r'));
}

#[test]
fn round_trip_revalidates() {
    let doc = RackDocument::from_yaml(SINE).unwrap();
    let out = doc.to_yaml().unwrap();
    validate_document(&out).expect("serialized document is schema-valid");
}

#[test]
fn wrong_format_rejected() {
    let text = SINE.replace("format: bitfiddle-rack", "format: something-else");
    assert!(RackDocument::from_yaml(&text).is_err());
}

#[test]
fn unknown_field_rejected() {
    let text = SINE.replace("format_version: 2", "format_version: 2\nbogus_field: 1");
    assert!(RackDocument::from_yaml(&text).is_err());
    assert!(validate_schema(&text).is_err());
}

#[test]
fn duplicate_module_name_rejected() {
    let text = SINE.replace("name: Main Output", "name: Main Oscillator");
    let err = validate_document(&text).unwrap_err();
    assert!(matches!(err, ValidationError::DuplicateModuleName(_)));
}

#[test]
fn overlapping_modules_rejected() {
    let text = SINE.replace("position: { x: 8, y: 0 }", "position: { x: 1, y: 0 }");
    let err = validate_document(&text).unwrap_err();
    assert!(matches!(err, ValidationError::Overlap(_, _)));
}

#[test]
fn unknown_selection_rejected() {
    let text = SINE.replace(
        "selected: []",
        "selected: [11111111-2222-3333-4444-555555555555]",
    );
    let err = validate_document(&text).unwrap_err();
    assert!(matches!(err, ValidationError::UnknownSelection(_)));
}

#[test]
fn input_state_signal_mismatch_rejected() {
    // Give the oscillator's note input a control-shaped state.
    let text = SINE.replace(
        "      note:\n        signal: note\n        manual_hz: 440\n        transpose_semitones: 0",
        "      note:\n        signal: control\n        baseline: 0\n        window: 1",
    );
    let err = validate_document(&text).unwrap_err();
    assert!(matches!(err, ValidationError::InputSignalMismatch { .. }));
}

#[test]
fn invalid_yaml_never_parses_partially() {
    let text = &SINE[..SINE.len() / 2];
    assert!(RackDocument::from_yaml(text).is_err());
}

#[test]
fn schema_rejects_bad_zoom() {
    let text = SINE.replace("zoom: 1", "zoom: 99");
    assert!(validate_schema(&text).is_err());
}

#[test]
fn schema_rejects_bad_signal_enum() {
    let text = SINE.replace("signal: audio", "signal: voltage");
    assert!(validate_schema(&text).is_err());
}
