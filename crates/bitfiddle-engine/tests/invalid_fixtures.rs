//! Invalid fixture harness: every fixture under fixtures/invalid must fail
//! validation with the documented reason and never mutate a graph
//! (PRD §18.5, §20 step 8).

use bitfiddle_engine::validate::{validate_document, ValidationError};

const FEEDBACK: &str = include_str!("../../../fixtures/invalid/feedback-cycle.bitfiddle.yaml");
const TWO_CLOCKS: &str = include_str!("../../../fixtures/invalid/clock-two-sources.bitfiddle.yaml");

#[test]
fn feedback_cycle_fixture_rejected_as_cycle() {
    let err = validate_document(FEEDBACK).unwrap_err();
    match err {
        ValidationError::Graph(g) => {
            assert!(matches!(
                g,
                bitfiddle_engine::graph::GraphError::Cycle { .. }
            ));
            // The rejection message identifies modules on the cycle path.
            let msg = g.to_string();
            assert!(msg.contains("Vol A") || msg.contains("Vol B"), "{msg}");
        }
        other => panic!("expected cycle rejection, got {other}"),
    }
}

#[test]
fn clock_two_sources_fixture_rejected() {
    let err = validate_document(TWO_CLOCKS).unwrap_err();
    match err {
        ValidationError::Graph(g) => assert!(matches!(
            g,
            bitfiddle_engine::graph::GraphError::ClockMultipleSources { .. }
        )),
        other => panic!("expected clock multi-source rejection, got {other}"),
    }
}

#[test]
fn invalid_fixtures_still_pass_schema() {
    // Both are schema-valid documents; only semantic/graph validation fails.
    bitfiddle_engine::validate::validate_schema(FEEDBACK).unwrap();
    bitfiddle_engine::validate::validate_schema(TWO_CLOCKS).unwrap();
}
