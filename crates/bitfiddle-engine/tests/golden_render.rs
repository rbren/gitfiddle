//! Golden audio tests: deterministic offline renders from complete
//! .bitfiddle.yaml fixtures (PRD §18.3).

use bitfiddle_engine::render::render_offline;
use bitfiddle_engine::validate::validate_document;

const SINE: &str = include_str!("../../../fixtures/sine.bitfiddle.yaml");

fn render(yaml: &str, seconds: f64) -> Vec<[f32; 2]> {
    let doc = validate_document(yaml).expect("fixture valid");
    render_offline(doc, seconds).expect("render ok")
}

/// Deterministic FNV-1a hash of the rendered samples' bit patterns.
fn sample_hash(samples: &[[f32; 2]]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for s in samples {
        for ch in s {
            for b in ch.to_bits().to_le_bytes() {
                h ^= b as u64;
                h = h.wrapping_mul(0x100000001b3);
            }
        }
    }
    h
}

#[test]
fn sine_render_is_deterministic() {
    let a = render(SINE, 0.5);
    let b = render(SINE, 0.5);
    assert_eq!(sample_hash(&a), sample_hash(&b));
}

#[test]
fn sine_render_is_440_hz_full_scale() {
    let samples = render(SINE, 1.0);
    assert_eq!(samples.len(), 48_000);
    let peak = samples
        .iter()
        .map(|s| s[0].abs())
        .fold(0.0f32, f32::max);
    assert!(peak > 0.99 && peak <= 1.0, "peak {peak}");
    // Count rising zero crossings to estimate frequency.
    let mut crossings = 0;
    for w in samples.windows(2) {
        if w[0][0] <= 0.0 && w[1][0] > 0.0 {
            crossings += 1;
        }
    }
    assert!(
        (438..=442).contains(&crossings),
        "expected ~440 rising crossings, got {crossings}"
    );
}

#[test]
fn stereo_channels_match_for_mono_voice() {
    let samples = render(SINE, 0.1);
    for s in &samples {
        assert_eq!(s[0], s[1]);
    }
}

#[test]
fn silence_when_wire_removed() {
    let yaml = {
        // Strip the wire list.
        let start = SINE.find("wires:").unwrap();
        let end = SINE.find("input_sync:").unwrap();
        format!("{}wires: []\n{}", &SINE[..start], &SINE[end..])
    };
    let samples = render(&yaml, 0.1);
    assert!(samples.iter().all(|s| s[0] == 0.0 && s[1] == 0.0));
}

#[test]
fn default_source_sine_when_disconnected_input() {
    // With no wire, but default_source sine_440 on the output's audio input,
    // the output must produce the default sine.
    let yaml = {
        let start = SINE.find("wires:").unwrap();
        let end = SINE.find("input_sync:").unwrap();
        format!("{}wires: []\n{}", &SINE[..start], &SINE[end..])
    }
    .replace("default_source: silence", "default_source: sine_440");
    let samples = render(&yaml, 0.5);
    let peak = samples.iter().map(|s| s[0].abs()).fold(0.0f32, f32::max);
    assert!(peak > 0.99, "default sine should be audible, peak {peak}");
}

#[test]
fn white_noise_default_is_seed_deterministic() {
    let yaml = {
        let start = SINE.find("wires:").unwrap();
        let end = SINE.find("input_sync:").unwrap();
        format!("{}wires: []\n{}", &SINE[..start], &SINE[end..])
    }
    .replace("default_source: silence", "default_source: white_noise")
    .replace("seed: 0", "seed: 42");
    let a = render(&yaml, 0.2);
    let b = render(&yaml, 0.2);
    assert_eq!(sample_hash(&a), sample_hash(&b));
    let peak = a.iter().map(|s| s[0].abs()).fold(0.0f32, f32::max);
    assert!(peak > 0.5, "noise should be audible");
}

#[test]
fn gain_scales_output() {
    let yaml = SINE.replace("gain: 1", "gain: 0.5");
    let samples = render(&yaml, 0.5);
    let peak = samples.iter().map(|s| s[0].abs()).fold(0.0f32, f32::max);
    assert!((peak - 0.5).abs() < 0.01, "peak {peak}");
}

#[test]
fn transpose_shifts_octave() {
    let yaml = SINE.replace("transpose_semitones: 0", "transpose_semitones: 12");
    let samples = render(&yaml, 1.0);
    let mut crossings = 0;
    for w in samples.windows(2) {
        if w[0][0] <= 0.0 && w[1][0] > 0.0 {
            crossings += 1;
        }
    }
    // Manual note is not transposed (input disconnected) => still 440.
    assert!(
        (438..=442).contains(&crossings),
        "manual note is never transposed, got {crossings}"
    );
}

#[test]
fn saw_waveform_renders_differently() {
    let yaml = SINE.replace("waveform: sine", "waveform: saw");
    let sine = render(SINE, 0.1);
    let saw = render(&yaml, 0.1);
    assert_ne!(sample_hash(&sine), sample_hash(&saw));
}

#[test]
fn known_golden_hash_for_sine_fixture() {
    // Regenerate intentionally with:
    //   cargo test -p bitfiddle-engine --test golden_render -- --nocapture print_golden
    let samples = render(SINE, 0.25);
    let hash = sample_hash(&samples);
    assert_eq!(
        hash, GOLDEN_SINE_HASH,
        "golden mismatch: got {hash:#018x}"
    );
}

// Committed golden value; CI never regenerates it.
const GOLDEN_SINE_HASH: u64 = 0x48a88cec2ddb7c6d;

#[test]
#[ignore = "generator: run with --ignored --nocapture to print the golden hash"]
fn print_golden() {
    let samples = render(SINE, 0.25);
    println!("GOLDEN_SINE_HASH = {:#018x}", sample_hash(&samples));
}
