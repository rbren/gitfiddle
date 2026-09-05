//! Signal merge rules and input-control application (PRD §6).
//!
//! For every input or sync group, processing order is normative:
//! 1. Collect output wires in saved order.
//! 2. Merge according to signal type.
//! 3. Apply the shared input control.
//! 4. Enforce the input's flavor-specific allowed range.
//! 5. Deliver the result to each consuming module.

use crate::signal::{SignalBlock, Voice, MAX_CHANNELS};

/// Merge Note sources: concatenate active channels in saved wire order,
/// capped at 16 channels; overflow channels are dropped (PRD §6.3, §6.1).
/// Returns `(merged, overflowed)`.
pub fn merge_note(sources: &[&SignalBlock]) -> (Vec<Vec<f32>>, bool) {
    let mut merged: Vec<Vec<f32>> = Vec::new();
    let mut overflow = false;
    for src in sources {
        if let SignalBlock::Note(chans) = src {
            for ch in chans {
                if merged.len() < MAX_CHANNELS {
                    merged.push(ch.clone());
                } else {
                    overflow = true;
                }
            }
        }
    }
    (merged, overflow)
}

/// Merge Audio sources: concatenate voices in saved wire order, capped at
/// 16 voices (PRD §6.4).
pub fn merge_audio(sources: &[&SignalBlock]) -> (Vec<Voice>, bool) {
    let mut merged: Vec<Voice> = Vec::new();
    let mut overflow = false;
    for src in sources {
        if let SignalBlock::Audio(voices) = src {
            for v in voices {
                if merged.len() < MAX_CHANNELS {
                    merged.push(v.clone());
                } else {
                    overflow = true;
                }
            }
        }
    }
    (merged, overflow)
}

/// Merge Control sources by channel-wise addition with mono broadcast and
/// clamping to `[-1, 1]` (PRD §6.5).
pub fn merge_control(sources: &[&SignalBlock], frames: usize) -> Vec<Vec<f32>> {
    let mut max_chans = 0usize;
    for src in sources {
        if let SignalBlock::Control(chans) = src {
            max_chans = max_chans.max(chans.len());
        }
    }
    if max_chans == 0 {
        return Vec::new();
    }
    let mut merged = vec![vec![0.0f32; frames]; max_chans];
    for src in sources {
        if let SignalBlock::Control(chans) = src {
            if chans.is_empty() {
                continue;
            }
            for (ci, out) in merged.iter_mut().enumerate() {
                let lane: &[f32] = if chans.len() == 1 {
                    // One-channel source broadcasts to every result channel.
                    &chans[0]
                } else if ci < chans.len() {
                    &chans[ci]
                } else {
                    continue; // missing channels contribute 0
                };
                for (o, s) in out.iter_mut().zip(lane.iter()) {
                    *o += *s;
                }
            }
        }
    }
    for lane in &mut merged {
        for s in lane.iter_mut() {
            *s = s.clamp(-1.0, 1.0);
        }
    }
    merged
}

/// Merge Gate sources with channel-wise OR and mono broadcast (PRD §6.6).
pub fn merge_gate(sources: &[&SignalBlock], frames: usize) -> Vec<Vec<bool>> {
    let mut max_chans = 0usize;
    for src in sources {
        if let SignalBlock::Gate(chans) = src {
            max_chans = max_chans.max(chans.len());
        }
    }
    if max_chans == 0 {
        return Vec::new();
    }
    let mut merged = vec![vec![false; frames]; max_chans];
    for src in sources {
        if let SignalBlock::Gate(chans) = src {
            if chans.is_empty() {
                continue;
            }
            for (ci, out) in merged.iter_mut().enumerate() {
                let lane: &[bool] = if chans.len() == 1 {
                    &chans[0]
                } else if ci < chans.len() {
                    &chans[ci]
                } else {
                    continue; // missing channels are off
                };
                for (o, s) in out.iter_mut().zip(lane.iter()) {
                    *o = *o || *s;
                }
            }
        }
    }
    merged
}

/// Apply the Control input's saved baseline/window:
/// `clamp(baseline + merged * window, -1, 1)` (PRD §6.5). When there are no
/// sources the input delivers its baseline on one channel.
pub fn apply_control_input(
    merged: Vec<Vec<f32>>,
    baseline: f32,
    window: f32,
    frames: usize,
) -> Vec<Vec<f32>> {
    if merged.is_empty() {
        return vec![vec![baseline.clamp(-1.0, 1.0); frames]];
    }
    merged
        .into_iter()
        .map(|lane| {
            lane.into_iter()
                .map(|s| (baseline + s * window).clamp(-1.0, 1.0))
                .collect()
        })
        .collect()
}

/// Apply the Note input control: manual note when disconnected, transpose in
/// semitones when connected (PRD §6.3).
pub fn apply_note_input(
    merged: Vec<Vec<f32>>,
    manual_hz: f32,
    transpose_semitones: f32,
    frames: usize,
) -> Vec<Vec<f32>> {
    if merged.is_empty() {
        return vec![vec![manual_hz; frames]];
    }
    let ratio = 2f32.powf(transpose_semitones / 12.0);
    merged
        .into_iter()
        .map(|lane| lane.into_iter().map(|hz| hz * ratio).collect())
        .collect()
}

/// Apply the Audio input's saved linear gain to every incoming voice after
/// polyphonic concatenation (PRD §6.4).
pub fn apply_audio_gain(mut voices: Vec<Voice>, gain: f32) -> Vec<Voice> {
    if (gain - 1.0).abs() < f32::EPSILON {
        return voices;
    }
    for v in &mut voices {
        for s in &mut v.left {
            *s *= gain;
        }
        if let Some(r) = &mut v.right {
            for s in r {
                *s *= gain;
            }
        }
    }
    voices
}

/// Apply the Gate input's manual latch: OR the saved latch with incoming
/// gates (PRD §6.6).
pub fn apply_gate_input(merged: Vec<Vec<bool>>, latched: bool, frames: usize) -> Vec<Vec<bool>> {
    if merged.is_empty() {
        return vec![vec![latched; frames]];
    }
    if !latched {
        return merged;
    }
    merged
        .into_iter()
        .map(|lane| lane.into_iter().map(|_| true).collect())
        .collect()
}
