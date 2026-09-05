# bitfiddle agent notes

Authoritative spec: `docs/PRD.md`. Where code and PRD differ, the PRD wins.

## Build & test

```sh
cargo test --workspace                 # engine + CLI tests
cargo clippy --workspace -- -D warnings
cargo fmt --all --check
cd frontend && npm install && npm run build && npm test
./run.sh                               # macOS app launch (Tauri dev)
```

## Layout

- `crates/bitfiddle-engine` — signal model (`signal.rs`), merge rules
  (`merge.rs`), typed DAG + sync groups + cycle checks (`graph.rs`), rack
  document model (`document.rs`), JSON Schema + semantic validation
  (`validate.rs`), execution/offline render (`render.rs`), built-ins
  (`modules/builtins.rs`).
- `crates/bitfiddle-cli` — `bitfiddle validate|render` headless commands.
- `crates/bitfiddle-app` — Tauri 2 shell (excluded from default workspace
  build; needs platform webview toolchains).
- `frontend/` — React/TS rack editor (Vite + Vitest).
- `schemas/` — normative JSON Schemas (rack, manifest, macro). The rack
  schema is embedded in the engine via `include_str!`.
- `fixtures/` — complete `.bitfiddle.yaml` racks used by tests and CI.
- `docs/traceability.md` — requirement coverage manifest; keep it current.

## Conventions

- Five digital signal types only: Clock, Note, Audio, Control, Gate. Never
  use voltage / V-oct / electrical terminology (PRD §3).
- Merge order is saved wire order; the input pipeline order in
  `render.rs::resolve_input` is normative (PRD §6.8) and shared by live and
  offline paths.
- Golden audio hashes are committed in `tests/golden_render.rs`; regenerate
  intentionally with the ignored `print_golden` test, never in CI.
- Serde types in `document.rs` must mirror `schemas/rack.schema.json`
  exactly, including enum value spellings (e.g. `sine_440`).
- Progressive commits with `Co-authored-by: openhands <openhands@all-hands.dev>`.
  Multiple agents may share this checkout: `git add` specific paths, never
  `git add -A`.
