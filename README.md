# bitfiddle

A desktop application for building audio systems by placing modules on an
infinite Rack and connecting their typed inputs and outputs. A Rack is both a
visual graph and a single, human-editable YAML document.

- **Shell:** Tauri 2
- **Engine:** Rust (real-time safe audio graph)
- **Frontend:** React + TypeScript

See [docs/PRD.md](docs/PRD.md) for the full product requirements document.

## Quick start (macOS)

```sh
./run.sh
```

`run.sh` installs or builds repository-local dependencies when needed, builds
extension UIs and DSP artifacts, and launches the Tauri app.

## Repository layout

```text
crates/
  bitfiddle-engine/   # signal model, typed DAG, merge rules, DSP modules,
                      # YAML persistence, validation, offline rendering
  bitfiddle-cli/      # headless CLI: load a Rack YAML, render WAV offline
  bitfiddle-app/      # Tauri 2 desktop shell (command layer + control thread)
frontend/             # React + TypeScript Rack editor
schemas/              # normative JSON Schemas (rack, manifest, macro)
fixtures/             # .bitfiddle.yaml Rack fixtures used by tests
docs/                 # PRD and architecture notes
```

## Signal types

bitfiddle uses five digital signal types — **Clock, Note, Audio, Control, and
Gate** — with typed ports, deterministic saved-wire-order merging, and up to 16
polyphonic channels per signal.

## Headless rendering

```sh
cargo run -p bitfiddle-cli -- render fixtures/sine.bitfiddle.yaml out.wav --seconds 2
```

Offline rendering uses the same graph, merge logic, and module implementations
as live playback, with a deterministic clock origin and deterministic seeds.

## Development

```sh
cargo test --workspace          # engine unit / integration / golden tests
cd frontend && npm test         # frontend component tests
```
