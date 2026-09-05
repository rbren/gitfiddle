//! bitfiddle engine: signal model, typed DAG, merge rules, built-in DSP
//! modules, Rack YAML persistence, validation, and offline rendering.
//!
//! See `docs/PRD.md` for the authoritative specification.

pub mod document;
pub mod graph;
pub mod manifest;
pub mod merge;
pub mod modules;
pub mod render;
pub mod signal;
pub mod validate;

pub const DEFAULT_SAMPLE_RATE: u32 = 48_000;
pub const DEFAULT_BLOCK_SIZE: usize = 128;
pub const MAX_CHANNELS: usize = 16;
pub const MAX_AUDIO_LANES: usize = 32;
pub const GRID_UNIT_PX: u32 = 64;
