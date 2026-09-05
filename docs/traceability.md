# Requirement traceability manifest

Status legend: **auto** = automated test exists; **partial** = partially
covered; **planned** = not yet implemented; **human** = requires human
acceptance sign-off (PRD §18.7).

| Requirement (PRD §) | Coverage | Where |
|---|---|---|
| §3 five signal types, no voltage terminology | auto | `signal.rs`, schema `signal` enum; docs reviewed |
| §5.1 default 48 kHz / 128 frames, f32, planar | auto | `lib.rs` constants; `golden_render.rs` |
| §5.1 clipping only at physical boundary | auto | `AudioOutput::mixdown` + `poly_fixture_validates_and_renders` |
| §5.1 NaN/inf replaced with silence + fault | auto | `sanitize_replaces_non_finite_with_zero` |
| §5.3 DAG after sync collapse, stable topo order | auto | `graph_rules.rs` topo/determinism tests |
| §5.3 cycle-checked before commit, atomic reject | auto | `would_create_cycle_detects_without_mutation` |
| §5.3 merge order = saved wire order | auto | `merge_rules.rs` concat-order tests |
| §5.3 latency compensation | planned | engine latency fields exist; compensation TBD |
| §5.4 device hot-plug, null output | planned | live device layer (cpal) TBD |
| §5.5 headless CLI renders YAML → WAV | auto | `bitfiddle-cli`, CI render smoke |
| §5.5 offline = live semantics, deterministic seeds | auto | `renders_are_deterministic_across_runs`, noise seed test |
| §6.1 same-type wiring only, fan-out, multi-source | auto | `signal_mismatch_is_rejected`, merge tests |
| §6.1 16-channel cap, saved-order overflow drop | auto | `note_polyphony_caps_at_16_and_reports_overflow` |
| §6.2 clock 0/1 source incl. sync groups | auto | `clock_input_rejects_second_source`, sync variant |
| §6.2 disconnected manual frequency; connected follows exactly | auto | `render.rs` clock input path |
| §6.3 note concat merge, manual note, semitone transpose | auto | merge + `transpose_shifts_octave` |
| §6.4 audio concat merge, mono→stereo copy, gain, defaults | auto | merge tests, `default_source_*`, `gain_scales_output` |
| §6.5 control add/broadcast/clamp, baseline+window | auto | control merge tests |
| §6.6 gate OR/broadcast, latch | auto | gate merge tests |
| §6.7 sync groups share sources/controls, identical member state | auto | `sync_group_shares_sources_across_members`, `DivergentSyncState` validation |
| §6.8 normative input pipeline order | auto | `render.rs::resolve_input` (single implementation for live+offline) |
| §7.3 automatic category assignment | auto | `registry.rs::category` |
| §7.4 module documentation | done | `docs/modules.md` |
| §7.6 panel geometry, host border, corners | partial | frontend implementation |
| §8 canvas, picker, selection, wires, undo/redo | partial | frontend implementation |
| §9.1 one-file rule, no side databases | auto | document model is complete rack state |
| §9.2 deterministic YAML formatting | auto | `yaml_round_trip_is_stable` |
| §9.2 atomic apply, invalid YAML keeps prior graph | partial | validation is atomic; editor flow in frontend |
| §9.6 normative schema installed and enforced | auto | `schemas/rack.schema.json`, `validate_schema` |
| §9.6 semantic validation (uniqueness, overlap, refs, cycles) | auto | `persistence.rs` |
| §10 macros: recursive expansion, budgets, cycles | partial | `macros.rs` subsystem (in progress) |
| §11 extensions: wasm-2/native-2 ABI, manifests | planned | manifest schema installed; hosts TBD |
| §12 keyboard system | partial | frontend implementation |
| §14 initial module set | auto | all 11 built-ins in `builtins.rs`; behavior tests in `module_behavior.rs` |
| §15 performance targets | human/planned | requires macOS baseline hardware |
| §17 YAML anchors/aliases rejected | auto | `from_yaml` pre-check |
| §18.3 golden audio with committed hashes | auto | `known_golden_hash_for_sine_fixture` |
| §18.7 release gates | partial | CI runs fmt/clippy/tests/fixtures/render |
| §19 M0 exit: schema-valid Rack renders deterministic audio offline | **met** | CI + golden tests |
| §20 final acceptance scenario | planned | requires M1–M4 completion |
