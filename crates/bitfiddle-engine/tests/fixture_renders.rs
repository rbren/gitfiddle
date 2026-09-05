//! Fixture-based render behavior tests (PRD §18.3 baseline cases).

use bitfiddle_engine::render::render_offline;
use bitfiddle_engine::validate::validate_document;

const POLY: &str = include_str!("../../../fixtures/poly-merge.bitfiddle.yaml");
const ADSR: &str = include_str!("../../../fixtures/adsr-gate.bitfiddle.yaml");

fn render(yaml: &str, seconds: f64) -> Vec<[f32; 2]> {
    let doc = validate_document(yaml).expect("fixture valid");
    render_offline(doc, seconds).expect("render ok")
}

#[test]
fn poly_fixture_validates_and_renders() {
    let samples = render(POLY, 0.5);
    assert_eq!(samples.len(), 24_000);
    let peak = samples.iter().map(|s| s[0].abs()).fold(0.0f32, f32::max);
    // Two full-scale voices at gain 0.5 sum to ~1.0 at instants of alignment.
    assert!(peak > 0.6, "expected audible poly mix, peak {peak}");
}

#[test]
fn poly_merge_concatenates_voices_before_gain() {
    // Removing the second wire halves the mixdown sum.
    let one_voice = {
        let yaml = POLY.replace(
            "  - id: 10000000-0000-4000-8000-000000000002
    signal: audio
    source: { module: bbbbbbbb-2222-4222-8222-222222222222, port: audio_out }
    target: { module: cccccccc-3333-4333-8333-333333333333, port: audio_in }
    order: 1
    waypoints: []
",
            "",
        );
        render(&yaml, 0.25)
    };
    let two_voices = render(POLY, 0.25);
    let rms = |s: &[[f32; 2]]| {
        (s.iter().map(|x| (x[0] * x[0]) as f64).sum::<f64>() / s.len() as f64).sqrt()
    };
    assert!(rms(&two_voices) > rms(&one_voice) * 1.3);
}

#[test]
fn adsr_latched_gate_produces_envelope_shaped_audio() {
    let samples = render(ADSR, 1.0);
    // Attack: early samples much quieter than post-attack samples.
    let early: f32 = samples[..240] // first 5 ms
        .iter()
        .map(|s| s[0].abs())
        .fold(0.0, f32::max);
    let later: f32 = samples[24_000..30_000] // 0.5s in: sustain at 0.6
        .iter()
        .map(|s| s[0].abs())
        .fold(0.0, f32::max);
    assert!(
        early < later,
        "attack should ramp: early {early} later {later}"
    );
    assert!((later - 0.6).abs() < 0.05, "sustain ~0.6, got {later}");
}

#[test]
fn adsr_unlatched_gate_is_silent() {
    let yaml = ADSR.replace("latched: true", "latched: false");
    let samples = render(&yaml, 0.25);
    let peak = samples.iter().map(|s| s[0].abs()).fold(0.0f32, f32::max);
    assert_eq!(peak, 0.0);
}

#[test]
fn renders_are_deterministic_across_runs() {
    let a = render(POLY, 0.3);
    let b = render(POLY, 0.3);
    assert_eq!(a, b);
    let c = render(ADSR, 0.3);
    let d = render(ADSR, 0.3);
    assert_eq!(c, d);
}
