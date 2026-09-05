# Architecture decisions

PRD §4.2 requires an explicit repository-recorded decision wherever the
implementation replaces or defers a required core library.

## AD-001: Deferred live-device and extension-host crates

**Status:** accepted (Phase 1, M0–M1)

The PRD requires `cpal` (device I/O), `symphonia` (file decoding), Wasmtime
(`wasm-2` DSP), and `libloading` (`native-2` DSP). The current milestone
delivers the deterministic headless engine, persistence, validation, and the
offline renderer — none of which touch physical devices or extension
binaries. Those crates are adopted unchanged when their subsystems land:

- `cpal` with the live audio thread (M0 exit requires a null/live device;
  the null device is implemented engine-side without a device dependency).
- `symphonia` with the Audio File Generator's host loader (the DSP core is
  in place and tested against injected sample buffers).
- Wasmtime (SIMD enabled) and `libloading` with the M2 extension hosts.

No replacement library was chosen for any of these; the dependency is only
deferred, not substituted.

## AD-002: `serde_yaml` for the canonical document

`serde_yaml` 0.9 is the YAML implementation. Determinism requirements
(two-space indent, stable key order, LF, one trailing newline, no anchors)
are satisfied by struct declaration order plus an explicit anchor/alias
pre-check in `document::RackDocument::from_yaml`. Anchors and aliases are
rejected before parse per PRD §17.

## AD-003: FNV-1a sample hashing for golden audio

Golden comparisons hash the exact `f32` bit patterns of rendered samples
with FNV-1a (64-bit). This is not a cryptographic hash; it is chosen for
zero dependencies and total determinism. Cases that cannot be bit-identical
across architectures must use bounded waveform/spectral metrics instead
(PRD §18.3) — none of the current built-ins need that.

## AD-004: Module DSP trait uses owned per-block buffers in Phase 1

The PRD ABI (§11.3) mandates planar fixed-stride buffers for `wasm-2` /
`native-2`. Built-in modules currently exchange `SignalBlock` values (Vec
per lane) through the control-path executor, which allocates. This is
acceptable only for the offline/headless renderer. The real-time audio
thread introduced with `cpal` must pre-plan buffers at graph publication
time and reuse them per block; the `DspModule` trait will grow a
planar-buffer entry point at that time, and the allocation tripwire
(PRD §5.2) will enforce it.
