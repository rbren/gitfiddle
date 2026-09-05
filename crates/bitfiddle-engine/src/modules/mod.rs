//! Built-in modules (PRD §14) and the module registry.

pub mod builtins;
pub mod registry;

use std::collections::HashMap;

use crate::signal::SignalBlock;

/// Per-block processing context.
pub struct ProcessCtx {
    pub sample_rate: u32,
    pub frames: usize,
    /// Absolute sample index of the first frame in this block, from the
    /// deterministic clock origin.
    pub start_sample: u64,
}

/// A DSP module instance. Inputs are pre-merged per the normative input
/// pipeline; outputs are keyed by stable port ID.
pub trait DspModule: Send {
    fn process(
        &mut self,
        ctx: &ProcessCtx,
        inputs: &HashMap<String, SignalBlock>,
    ) -> HashMap<String, SignalBlock>;

    fn reset(&mut self) {}

    /// Declared latency in samples (PRD §5.3).
    fn latency_samples(&self) -> u32 {
        0
    }
}
