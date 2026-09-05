//! The five bitfiddle signal types and their per-block buffer shapes.
//!
//! These are digital signal types, not electrical signals (PRD §3, §6).

use serde::{Deserialize, Serialize};

pub const MAX_CHANNELS: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SignalType {
    Clock,
    Note,
    Audio,
    Control,
    Gate,
}

impl SignalType {
    pub fn name(self) -> &'static str {
        match self {
            SignalType::Clock => "clock",
            SignalType::Note => "note",
            SignalType::Audio => "audio",
            SignalType::Control => "control",
            SignalType::Gate => "gate",
        }
    }
}

/// One audio voice: left samples plus optional right samples.
/// A mono voice supplies only its left lane; the host copies left to right
/// when a stereo consumer reads it (PRD §6.4).
#[derive(Debug, Clone, PartialEq)]
pub struct Voice {
    pub left: Vec<f32>,
    /// `None` marks a mono voice.
    pub right: Option<Vec<f32>>,
}

impl Voice {
    pub fn mono(left: Vec<f32>) -> Self {
        Voice { left, right: None }
    }

    pub fn stereo(left: Vec<f32>, right: Vec<f32>) -> Self {
        Voice {
            left,
            right: Some(right),
        }
    }

    pub fn right_lane(&self) -> &[f32] {
        self.right.as_deref().unwrap_or(&self.left)
    }
}

/// A per-block signal value flowing through one port.
#[derive(Debug, Clone, PartialEq)]
pub enum SignalBlock {
    /// Phase in `[0, 2π)` per sample; `None` when no clock source is active.
    Clock(Option<Vec<f32>>),
    /// One frequency lane (Hz) per active channel.
    Note(Vec<Vec<f32>>),
    /// 0–16 polyphonic voices.
    Audio(Vec<Voice>),
    /// One normalized `[-1, 1]` lane per active channel.
    Control(Vec<Vec<f32>>),
    /// One boolean lane per active channel.
    Gate(Vec<Vec<bool>>),
}

impl SignalBlock {
    pub fn signal_type(&self) -> SignalType {
        match self {
            SignalBlock::Clock(_) => SignalType::Clock,
            SignalBlock::Note(_) => SignalType::Note,
            SignalBlock::Audio(_) => SignalType::Audio,
            SignalBlock::Control(_) => SignalType::Control,
            SignalBlock::Gate(_) => SignalType::Gate,
        }
    }

    pub fn empty(signal: SignalType) -> Self {
        match signal {
            SignalType::Clock => SignalBlock::Clock(None),
            SignalType::Note => SignalBlock::Note(Vec::new()),
            SignalType::Audio => SignalBlock::Audio(Vec::new()),
            SignalType::Control => SignalBlock::Control(Vec::new()),
            SignalType::Gate => SignalBlock::Gate(Vec::new()),
        }
    }

    pub fn active_channels(&self) -> usize {
        match self {
            SignalBlock::Clock(p) => usize::from(p.is_some()),
            SignalBlock::Note(c) => c.len(),
            SignalBlock::Audio(v) => v.len(),
            SignalBlock::Control(c) => c.len(),
            SignalBlock::Gate(c) => c.len(),
        }
    }

    /// Replace NaN and infinity with zero/silence at a module boundary
    /// (PRD §5.1). Returns true when any value was sanitized.
    pub fn sanitize(&mut self) -> bool {
        fn scrub(lane: &mut [f32]) -> bool {
            let mut faulted = false;
            for s in lane.iter_mut() {
                if !s.is_finite() {
                    *s = 0.0;
                    faulted = true;
                }
            }
            faulted
        }
        match self {
            SignalBlock::Clock(Some(lane)) => scrub(lane),
            SignalBlock::Clock(None) => false,
            SignalBlock::Note(chans) | SignalBlock::Control(chans) => {
                let mut f = false;
                for lane in chans {
                    f |= scrub(lane);
                }
                f
            }
            SignalBlock::Audio(voices) => {
                let mut f = false;
                for v in voices {
                    f |= scrub(&mut v.left);
                    if let Some(r) = &mut v.right {
                        f |= scrub(r);
                    }
                }
                f
            }
            SignalBlock::Gate(_) => false,
        }
    }
}

pub const CLOCK_HZ_MIN: f64 = 0.0;
pub const CLOCK_HZ_MAX: f64 = 40.0;
pub const NOTE_HZ_MIN: f64 = 20.0;
pub const NOTE_HZ_MAX: f64 = 20_000.0;
