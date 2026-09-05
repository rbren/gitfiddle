//! Signal merge rule tests (PRD §6, §18.1).

use bitfiddle_engine::merge::*;
use bitfiddle_engine::signal::{SignalBlock, Voice};

fn note(chans: Vec<Vec<f32>>) -> SignalBlock {
    SignalBlock::Note(chans)
}

#[test]
fn note_sources_concatenate_in_saved_wire_order() {
    let a = note(vec![vec![440.0; 4], vec![550.0; 4]]);
    let b = note(vec![vec![660.0; 4]]);
    let (merged, overflow) = merge_note(&[&a, &b]);
    assert!(!overflow);
    assert_eq!(merged.len(), 3);
    assert_eq!(merged[0][0], 440.0);
    assert_eq!(merged[1][0], 550.0);
    assert_eq!(merged[2][0], 660.0);
}

#[test]
fn note_polyphony_caps_at_16_and_reports_overflow() {
    let a = note((0..10).map(|i| vec![100.0 + i as f32; 2]).collect());
    let b = note((0..10).map(|i| vec![200.0 + i as f32; 2]).collect());
    let (merged, overflow) = merge_note(&[&a, &b]);
    assert!(overflow);
    assert_eq!(merged.len(), 16);
    // Retained channels follow saved wire order then source-channel order.
    assert_eq!(merged[0][0], 100.0);
    assert_eq!(merged[9][0], 109.0);
    assert_eq!(merged[10][0], 200.0);
    assert_eq!(merged[15][0], 205.0);
}

#[test]
fn audio_sources_concatenate_voices_not_mixed() {
    let a = SignalBlock::Audio(vec![Voice::mono(vec![0.5; 4])]);
    let b = SignalBlock::Audio(vec![Voice::mono(vec![-0.25; 4])]);
    let (merged, overflow) = merge_audio(&[&a, &b]);
    assert!(!overflow);
    assert_eq!(merged.len(), 2);
    assert_eq!(merged[0].left[0], 0.5);
    assert_eq!(merged[1].left[0], -0.25);
}

#[test]
fn mono_voice_right_lane_copies_left() {
    let v = Voice::mono(vec![0.7; 3]);
    assert_eq!(v.right_lane(), &[0.7, 0.7, 0.7]);
    let s = Voice::stereo(vec![0.1; 2], vec![0.9; 2]);
    assert_eq!(s.right_lane(), &[0.9, 0.9]);
}

#[test]
fn control_merges_by_channelwise_addition_with_broadcast_and_clamp() {
    // One-channel source broadcasts to every result channel.
    let mono = SignalBlock::Control(vec![vec![0.5; 4]]);
    let stereo = SignalBlock::Control(vec![vec![0.25; 4], vec![0.75; 4]]);
    let merged = merge_control(&[&mono, &stereo], 4);
    assert_eq!(merged.len(), 2);
    assert_eq!(merged[0][0], 0.75); // 0.5 + 0.25
    assert_eq!(merged[1][0], 1.0); // 0.5 + 0.75 = 1.25 clamped
}

#[test]
fn control_missing_channels_contribute_zero() {
    let two = SignalBlock::Control(vec![vec![0.1; 2], vec![0.2; 2]]);
    let three = SignalBlock::Control(vec![vec![0.0; 2], vec![0.0; 2], vec![0.3; 2]]);
    let merged = merge_control(&[&two, &three], 2);
    assert_eq!(merged.len(), 3);
    assert!((merged[2][0] - 0.3).abs() < 1e-6); // two-channel source adds 0 here
}

#[test]
fn control_input_applies_baseline_window_and_clamps() {
    let merged = vec![vec![0.5f32; 2]];
    let out = apply_control_input(merged, 0.2, 1.0, 2);
    assert!((out[0][0] - 0.7).abs() < 1e-6);
    let clamped = apply_control_input(vec![vec![1.0f32; 1]], 0.5, 1.0, 1);
    assert_eq!(clamped[0][0], 1.0);
}

#[test]
fn disconnected_control_input_delivers_baseline() {
    let out = apply_control_input(Vec::new(), -0.3, 1.0, 3);
    assert_eq!(out.len(), 1);
    assert!((out[0][0] + 0.3).abs() < 1e-6);
}

#[test]
fn gate_merges_with_channelwise_or_and_broadcast() {
    let mono = SignalBlock::Gate(vec![vec![true, false]]);
    let stereo = SignalBlock::Gate(vec![vec![false, false], vec![false, true]]);
    let merged = merge_gate(&[&mono, &stereo], 2);
    assert_eq!(merged.len(), 2);
    assert_eq!(merged[0], vec![true, false]);
    assert_eq!(merged[1], vec![true, true]);
}

#[test]
fn gate_latch_ors_with_incoming() {
    let merged = vec![vec![false, true]];
    let out = apply_gate_input(merged, true, 2);
    assert_eq!(out[0], vec![true, true]);
    let out2 = apply_gate_input(vec![vec![false, true]], false, 2);
    assert_eq!(out2[0], vec![false, true]);
}

#[test]
fn disconnected_gate_delivers_latch() {
    let out = apply_gate_input(Vec::new(), true, 2);
    assert_eq!(out, vec![vec![true, true]]);
}

#[test]
fn note_transpose_is_equal_tempered_semitones() {
    let merged = vec![vec![440.0f32; 1]];
    let out = apply_note_input(merged, 440.0, 12.0, 1);
    assert!((out[0][0] - 880.0).abs() < 0.01);
    let down = apply_note_input(vec![vec![440.0f32; 1]], 440.0, -12.0, 1);
    assert!((down[0][0] - 220.0).abs() < 0.01);
}

#[test]
fn disconnected_note_produces_manual_note() {
    let out = apply_note_input(Vec::new(), 261.63, 7.0, 2);
    assert_eq!(out.len(), 1);
    // Transposition does not apply to the manual note itself.
    assert!((out[0][0] - 261.63).abs() < 1e-3);
}

#[test]
fn audio_gain_applies_to_every_voice_after_concat() {
    let voices = vec![
        Voice::mono(vec![1.0; 2]),
        Voice::stereo(vec![0.5; 2], vec![-0.5; 2]),
    ];
    let out = apply_audio_gain(voices, 2.0);
    assert_eq!(out[0].left[0], 2.0);
    assert_eq!(out[1].left[0], 1.0);
    assert_eq!(out[1].right.as_ref().unwrap()[0], -1.0);
}

#[test]
fn sanitize_replaces_non_finite_with_zero() {
    let mut block = SignalBlock::Audio(vec![Voice::mono(vec![f32::NAN, 1.0, f32::INFINITY])]);
    let faulted = block.sanitize();
    assert!(faulted);
    if let SignalBlock::Audio(v) = &block {
        assert_eq!(v[0].left, vec![0.0, 1.0, 0.0]);
    } else {
        panic!("wrong variant");
    }
}
