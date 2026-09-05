//! Behavior tests for individual built-in modules and the automatic
//! category rule (PRD §7.3, §14).

use std::collections::HashMap;

use bitfiddle_engine::modules::builtins::{instantiate, spec};
use bitfiddle_engine::modules::registry::Category;
use bitfiddle_engine::modules::ProcessCtx;
use bitfiddle_engine::signal::{SignalBlock, Voice};

fn ctx(frames: usize) -> ProcessCtx {
    ProcessCtx {
        sample_rate: 48_000,
        frames,
        start_sample: 0,
    }
}

fn params(json: serde_json::Value) -> serde_json::Map<String, serde_json::Value> {
    json.as_object().unwrap().clone()
}

#[test]
fn automatic_categories_follow_first_matching_rule() {
    assert_eq!(spec("app.qwerty").unwrap().category(), Category::Sequencer);
    assert_eq!(spec("app.clock").unwrap().category(), Category::Clock);
    assert_eq!(spec("app.adsr").unwrap().category(), Category::Logic);
    assert_eq!(
        spec("app.oscillator").unwrap().category(),
        Category::Generator
    );
    assert_eq!(spec("app.noise").unwrap().category(), Category::Generator);
    assert_eq!(
        spec("app.audio_output").unwrap().category(),
        Category::Output
    );
    assert_eq!(spec("app.scope").unwrap().category(), Category::Output);
    assert_eq!(spec("app.volume").unwrap().category(), Category::Effect);
    assert_eq!(spec("app.eq").unwrap().category(), Category::Effect);
    assert_eq!(spec("app.mixer8").unwrap().category(), Category::Mixer);
    assert_eq!(
        spec("app.audio_file").unwrap().category(),
        Category::Generator
    );
}

#[test]
fn all_module_dimensions_are_multiples_of_four_units() {
    for id in bitfiddle_engine::modules::builtins::all_type_ids() {
        let s = spec(id).unwrap();
        assert_eq!(s.width_units % 4, 0, "{id} width");
        assert_eq!(s.height_units % 4, 0, "{id} height");
        // A side must fit its ports in non-corner tiles (PRD §7.6).
        let left_ports = s
            .inputs
            .iter()
            .filter(|p| {
                matches!(
                    p.signal,
                    bitfiddle_engine::signal::SignalType::Note
                        | bitfiddle_engine::signal::SignalType::Gate
                        | bitfiddle_engine::signal::SignalType::Audio
                )
            })
            .count() as u32;
        assert!(
            s.height_units >= left_ports + 2 || left_ports == 0,
            "{id}: {left_ports} left ports need height"
        );
    }
}

#[test]
fn eq_flat_settings_pass_audio_unchanged() {
    let mut eq = instantiate("app.eq", &params(serde_json::json!({}))).unwrap();
    let input: Vec<f32> = (0..64).map(|i| (i as f32 / 64.0) - 0.5).collect();
    let inputs = HashMap::from([(
        "audio_in".to_string(),
        SignalBlock::Audio(vec![Voice::mono(input.clone())]),
    )]);
    let out = eq.process(&ctx(64), &inputs);
    if let Some(SignalBlock::Audio(v)) = out.get("audio_out") {
        for (a, b) in v[0].left.iter().zip(input.iter()) {
            assert!((a - b).abs() < 1e-5);
        }
    } else {
        panic!("no audio out");
    }
}

#[test]
fn eq_low_boost_amplifies_low_frequency() {
    let mut eq = instantiate("app.eq", &params(serde_json::json!({ "low_db": 12.0 }))).unwrap();
    // 50 Hz sine at 48 kHz, several blocks to settle the filter.
    let sr = 48_000f32;
    let mut peak_in = 0.0f32;
    let mut peak_out = 0.0f32;
    let mut phase = 0.0f32;
    for _block in 0..40 {
        let mut lane = Vec::with_capacity(128);
        for _ in 0..128 {
            lane.push(phase.sin() * 0.25);
            phase += std::f32::consts::TAU * 50.0 / sr;
        }
        peak_in = peak_in.max(lane.iter().fold(0.0f32, |m, s| m.max(s.abs())));
        let inputs = HashMap::from([(
            "audio_in".to_string(),
            SignalBlock::Audio(vec![Voice::mono(lane)]),
        )]);
        let out = eq.process(&ctx(128), &inputs);
        if let Some(SignalBlock::Audio(v)) = out.get("audio_out") {
            peak_out = peak_out.max(v[0].left.iter().fold(0.0f32, |m, s| m.max(s.abs())));
        }
    }
    assert!(
        peak_out > peak_in * 2.0,
        "low shelf +12 dB should ~4x a 50 Hz tone: in {peak_in} out {peak_out}"
    );
}

fn gate_block(pattern: &[bool]) -> SignalBlock {
    SignalBlock::Gate(vec![pattern.to_vec()])
}

fn audio_file(mode: &str) -> Box<dyn bitfiddle_engine::modules::DspModule> {
    instantiate(
        "app.audio_file",
        &params(serde_json::json!({
            "mode": mode,
            "test_samples": [0.1, 0.2, 0.3, 0.4]
        })),
    )
    .unwrap()
}

fn left_of(out: &HashMap<String, SignalBlock>) -> Vec<f32> {
    match out.get("audio_out") {
        Some(SignalBlock::Audio(v)) if !v.is_empty() => v[0].left.clone(),
        _ => Vec::new(),
    }
}

#[test]
fn audio_file_one_shot_plays_once_per_playthrough() {
    let mut m = audio_file("one_shot");
    // Gate held on the whole block: play the 4 samples once, then silence.
    let out = m.process(
        &ctx(8),
        &HashMap::from([("gate".into(), gate_block(&[true; 8]))]),
    );
    assert_eq!(left_of(&out), vec![0.1, 0.2, 0.3, 0.4, 0.0, 0.0, 0.0, 0.0]);
}

#[test]
fn audio_file_retrigger_restarts_on_each_rising_edge() {
    let mut m = audio_file("retrigger");
    let pattern = [true, true, false, true, true, true, true, true];
    let out = m.process(
        &ctx(8),
        &HashMap::from([("gate".into(), gate_block(&pattern))]),
    );
    // Restarts at frame 3.
    assert_eq!(left_of(&out), vec![0.1, 0.2, 0.3, 0.1, 0.2, 0.3, 0.4, 0.0]);
}

#[test]
fn audio_file_loop_repeats_while_gate_held() {
    let mut m = audio_file("loop");
    let out = m.process(
        &ctx(10),
        &HashMap::from([("gate".into(), gate_block(&[true; 10]))]),
    );
    assert_eq!(
        left_of(&out),
        vec![0.1, 0.2, 0.3, 0.4, 0.1, 0.2, 0.3, 0.4, 0.1, 0.2]
    );
}

#[test]
fn audio_file_missing_file_is_silent_not_fatal() {
    let mut m = instantiate("app.audio_file", &params(serde_json::json!({}))).unwrap();
    let out = m.process(
        &ctx(4),
        &HashMap::from([("gate".into(), gate_block(&[true; 4]))]),
    );
    match out.get("audio_out") {
        Some(SignalBlock::Audio(v)) => assert!(v.is_empty()),
        _ => panic!("expected audio out"),
    }
}

#[test]
fn mixer_mute_and_solo() {
    let mut m = instantiate(
        "app.mixer8",
        &params(serde_json::json!({ "mute_1": true, "gain_0": 0.5 })),
    )
    .unwrap();
    let inputs = HashMap::from([
        (
            "in_0".to_string(),
            SignalBlock::Audio(vec![Voice::mono(vec![1.0; 4])]),
        ),
        (
            "in_1".to_string(),
            SignalBlock::Audio(vec![Voice::mono(vec![1.0; 4])]),
        ),
    ]);
    let out = m.process(&ctx(4), &inputs);
    if let Some(SignalBlock::Audio(v)) = out.get("audio_out") {
        assert_eq!(v.len(), 1, "muted input contributes no voice");
        assert_eq!(v[0].left[0], 0.5, "gain applied");
    } else {
        panic!("no out");
    }

    let mut solo =
        instantiate("app.mixer8", &params(serde_json::json!({ "solo_1": true }))).unwrap();
    let inputs2 = HashMap::from([
        (
            "in_0".to_string(),
            SignalBlock::Audio(vec![Voice::mono(vec![0.25; 4])]),
        ),
        (
            "in_1".to_string(),
            SignalBlock::Audio(vec![Voice::mono(vec![0.75; 4])]),
        ),
    ]);
    let out2 = solo.process(&ctx(4), &inputs2);
    if let Some(SignalBlock::Audio(v)) = out2.get("audio_out") {
        assert_eq!(v.len(), 1, "solo excludes non-soloed inputs");
        assert_eq!(v[0].left[0], 0.75);
    } else {
        panic!("no out");
    }
}
