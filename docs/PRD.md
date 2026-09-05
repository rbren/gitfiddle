# bitfiddle Phase 1 — Core Rack Product Requirements Document

**Status:** Final implementation specification  
**Product:** Greenfield desktop application for modular audio synthesis  
**Platforms:** macOS first; architecture must remain portable to Windows and Linux  
**Shell:** Tauri 2  
**Backend and audio engine:** Rust  
**Frontend:** React + TypeScript

This document is authoritative for v2. Where it differs from the existing dj-station Rack, this document wins.

---

## 1. Product summary

bitfiddle is a desktop application for building audio systems by placing modules on an infinite Rack and connecting their typed inputs and outputs. A Rack is both a visual graph and a single, human-editable YAML document. Editing either representation updates the other.

The product must support:

- Built-in modules compiled into the engine.
- Independently distributed WASM and trusted native extension modules.
- A typed, polyphonic signal graph.
- Mouse, trackpad, and complete keyboard operation.
- Reusable global macros made from Rack subgraphs.
- Real-time audio with strict real-time safety.
- Deterministic persistence, undo/redo, autosave, recovery, and schema migration.
- Objective automated verification, including offline audio rendering.

The app must start on macOS from the repository root with:

```sh
./run.sh
```

`run.sh` must install or build repository-local dependencies when needed, build extension UIs and DSP artifacts, and launch the Tauri app. It must not require undocumented manual setup.

---

## 2. Goals and non-goals

### 2.1 Goals

1. Make typed audio graphs understandable and fast to edit.
2. Make every persistent user-authored Rack detail visible and editable in YAML.
3. Make every Rack operation possible without a mouse.
4. Keep the audio thread deterministic, bounded, and free of allocation and blocking.
5. Let extension authors add DSP and distinctive interfaces without rebuilding the app.
6. Preserve a working Rack when a module, UI, device, or saved document is unavailable or invalid.
7. Make automated agents able to prove behavior through schemas, APIs, offline rendering, and UI tests.

### 2.2 Non-goals for v2

- Hosting VST, Audio Unit, CLAP, or other third-party plugin formats.
- Recording a multitrack timeline.
- A piano-roll editor.
- Cloud synchronization or collaboration.
- Sandboxing native DSP modules. Native modules are trusted code.
- Generic measurement-unit metadata or automatic unit conversion.
- Emulating electrical modular-synth behavior or terminology.
- Feedback graphs. Signal cycles are not supported.
- Persisting transient runtime observations such as hover state, live telemetry samples, open menus, in-progress pointer gestures, or the process-global clock phase.

---

## 3. Product language and prohibited legacy concepts

The product uses five signal types: **Clock, Note, Audio, Control, and Gate**.

These are digital signal types, not electrical signals. The product must not describe signals as voltages, use V/oct terminology, show electrical calibration, or expose a generic system of units and display conversions. Module manifests must not contain generic `unit`, `display_map`, or conversion fields.

Signal-specific representations are allowed because they are part of the signal itself:

- Clock may be shown as frequency and phase.
- Note may be shown as frequency or musical pitch notation.
- Audio is shown as normalized amplitude and signal visualizations.
- Control is a normalized number.
- Gate is on or off.

A module that intentionally translates one signal type into another must do so as explicit DSP with a typed input and typed output. The host never performs implicit type conversion.

---

## 4. Architecture

### 4.1 Processes and responsibilities

- **React frontend:** Rack rendering, YAML editor, menus, dialogs, keyboard command system, custom module UI hosting, and visualization of engine telemetry.
- **Tauri command layer:** Narrow typed IPC API. It validates requests and delegates work; it never performs real-time audio processing.
- **Rust control thread:** Owns the canonical live graph, document synchronization, module discovery, compilation, file I/O, history, and preparation of immutable audio-thread updates.
- **Rust audio thread:** Processes the current immutable graph and publishes bounded telemetry. It never calls into the webview or filesystem.
- **Background workers:** Extension compilation, audio decoding, waveform preparation, schema validation that is too expensive for the UI, and other non-real-time work.

The Rust graph is the canonical live execution state. The YAML document is the canonical persistent state. Applying YAML is transactional: a fully parsed and validated candidate document becomes one graph edit; invalid YAML never partially changes the live graph.

### 4.2 Required core crates/libraries

- Tauri 2 for the desktop shell.
- React and TypeScript for the frontend.
- `cpal` for physical audio device I/O.
- `symphonia` for file decoding.
- `serde`, `serde_yaml`, and `serde_json` for document and schema-facing data.
- Wasmtime with SIMD enabled for WASM DSP modules.
- `libloading` for trusted native DSP modules.

Equivalent replacements require an explicit architecture decision recorded in the repository.

### 4.3 Data root

All application-managed persistent data must live below one application data root:

```text
<data-root>/
  autosave/
  extensions/
  macros/
  schemas/
  cache/
```

User Rack files may be saved anywhere through the native file dialog. The app must not scatter persistent state across unrelated platform directories or browser local storage.

---

## 5. Real-time audio engine

### 5.1 Default engine configuration

- Default sample rate: **48,000 Hz**.
- Default block size: **128 frames**.
- Both are configurable when supported by the selected device.
- Maximum polyphony per polyphonic signal: **16 channels**.
- Internal sample type: `f32`.
- Internal processing is planar.
- Audio is clipped only at the physical output boundary. Modules and graph merges must safely tolerate finite values outside the nominal range.
- NaN and infinity emitted by a module are replaced with silence/zero at that module boundary and reported as a module fault.

The default device, sample rate, and block size are Rack engine state. Physical device and channel mappings are state on each Audio Output module. All are saved in the Rack YAML.

### 5.2 Real-time safety

On the audio thread there must be:

- No heap allocation or deallocation.
- No locks, waits, sleeps, or blocking synchronization.
- No filesystem, network, logging, device enumeration, or IPC calls.
- No panics crossing the audio callback.
- No destruction of heap-owning graph/module state.

Graph and parameter edits are prepared on the control thread and transferred through bounded lock-free queues. Graph programs are immutable after publication. Replaced programs and modules are returned to a non-real-time garbage queue and dropped off the audio thread.

A debug allocation tripwire and a long-running stress test must enforce these requirements.

### 5.3 Graph execution

- The graph is a directed acyclic graph after input-sync groups are collapsed.
- Every new output-to-input connection is cycle-checked before commit.
- A connection that would create a cycle is rejected atomically and the UI identifies the path that would form the cycle.
- Modules execute in stable topological order.
- Reordering unrelated modules must not change merge order; merge order is the saved wire order.
- All five signal types are processed at audio rate in v2. The engine may optimize provably slow paths later only if behavior remains sample-equivalent.
- Declared module latency is tracked. The engine compensates parallel paths so signals mixed at an input are time-aligned. The physical output reports total graph latency.

### 5.4 Device behavior

- Device enumeration and hot-plug happen off the audio thread.
- If the selected device disappears, the graph continues on a null output clock when possible, the Rack remains editable, and a persistent error is shown.
- Reconnecting the same stable device ID restores output automatically.
- If the saved sample rate or block size is unsupported, the app proposes the nearest supported configuration and does not silently rewrite the Rack.
- Each Audio Output module selects a stable physical device ID and maps its logical inputs to physical device channels. Multiple Audio Output modules and multiple simultaneous devices are allowed when the platform supports them.

### 5.5 Offline rendering

The engine must expose a headless API and CLI that load one Rack YAML file and render a bounded duration to WAV faster than real time. Offline rendering uses the same graph, merge logic, module implementations, and sample rate as live playback, with a deterministic clock origin and deterministic random seeds unless the Rack explicitly stores another seed.

---

## 6. Signal model

### 6.1 Common rules

- Every port has exactly one signal type.
- Output-to-input wires and input-sync wires may connect only ports of the same signal type.
- Output-to-output wires are invalid.
- Outputs may fan out to any number of compatible inputs.
- Inputs may receive multiple output wires except Clock inputs, which may receive at most one Clock source across their entire sync group.
- Polyphonic signals carry an active channel count from 0 through 16.
- Channel ordering is stable and observable.
- When merged polyphony would exceed 16 channels, channels are retained in saved wire order and source-channel order; overflow channels are dropped and the input reports an overflow warning.
- The host does not convert between signal types.
- Each wire has a stable ID and saved order.

### 6.2 Clock

**Shape:** one channel.  
**Native representation:** phase in `[0, 2π)`, evaluated per sample. A wrap from high phase to low phase is a tick.  
**Frequency range:** `[0, 40]` Hz.  
**Phase origin:** derived from one monotonic timestamp captured at application start and shared by every Clock module. It is runtime-derived, not persisted.

Clock sources cannot combine. A Clock input or synchronized Clock input group may have zero or one output source. Attempts to add a second source are rejected.

A disconnected Clock input has a manual frequency control. A connected Clock input follows the incoming frequency and phase exactly; it has no host-level multiplier, offset, or transform. Any rate multiplication or division is explicit module behavior.

**Wire rendering:** dashed with equal dash and gap lengths. A moving tick pattern advances with Clock phase. Clock ports are on the top for inputs and bottom for outputs.

### 6.3 Note

**Shape:** 0–16 channels.  
**Native representation:** frequency in Hz for each active channel.  
**Range:** `[20, 20,000]` Hz.

Multiple Note sources combine polyphonically by concatenating their active channels in saved wire order. They are not averaged or summed.

A disconnected Note input produces one manually selected note. A connected Note input transposes every incoming channel. Ordinary dragging changes transposition in equal-tempered semitone steps; Command/Ctrl-drag changes it continuously for microtonal adjustment. The saved transposition is represented as a real number of semitones, not as a generic conversion setting.

The user can switch Note presentation between:

- Frequency.
- Octave + pitch class + microtone offset.

This is fixed Note UI behavior, not a general unit/conversion framework.

**Wire rendering:** one solid band per active channel. Hue identifies pitch class in 30-degree steps, lightness identifies octave, and saturation decreases as the note moves farther from the nearest equal-tempered pitch. Note inputs are on the left and outputs on the right.

### 6.4 Audio

**Shape:** 0–16 polyphonic voices, each containing left and right samples. The ABI exposes 32 lanes in `voice0-left, voice0-right, voice1-left, voice1-right, …` order and a 16-bit mono mask per Audio port.  
**Native representation:** normalized `f32` sample amplitude. Nominal full scale is `[-1, 1]`; finite internal values may exceed that range until the physical output boundary.

Multiple Audio sources combine polyphonically by concatenating voices in saved wire order. They are not mixed into fewer voices at the connection point. A source that marks a voice as mono supplies only its left lane; the host copies that lane to right when a stereo consumer reads it.

A disconnected Audio input opens a source menu rather than showing a numeric control. Required built-in defaults are:

- Silence.
- White noise.
- 440 Hz sine.
- 440 Hz saw.
- 440 Hz triangle.
- 440 Hz square.

The selected default source and its deterministic random seed are Rack state.

A connected Audio input has a saved linear gain control. The default is `1`; normal dragging changes gain in quantized steps and Command/Ctrl-drag changes it continuously. Gain is applied independently to every incoming voice after polyphonic concatenation.

**Wire rendering:** solid black. Width eases from 2 px at silence to 12 px at full-scale recent amplitude. The UI samples a fully combined visual amplitude at 10 Hz; it does not draw each voice separately. Audio inputs are on the left and outputs on the right.

### 6.5 Control

**Shape:** 0–16 channels.  
**Native representation:** normalized `f32` values.  
**Nominal range:** `[-1, 1]`.

Multiple Control sources combine by channel-wise addition:

1. The result channel count is the maximum active channel count of all sources.
2. A one-channel source broadcasts to every result channel.
3. A multi-channel source contributes its matching channel; missing channels contribute `0`.
4. Sources are added in saved wire order.
5. The merged value is clamped to `[-1, 1]`.

Every Control input has a saved **baseline** and **window**. The delivered value is:

```text
clamp(baseline + merged * window, -1, 1)
```

When disconnected, the input delivers its baseline. Normal dragging changes baseline; Command/Ctrl-drag changes window. The control must show baseline, window, and current delivered value simultaneously.

**Wire rendering:** one line per active channel. One channel is 5 px. Two through eight channels use 2 px lines with 2 px spacing. Nine through sixteen use 1 px lines with 1 px spacing. Negative values use a muted red, positive values a muted green, neutral values gray, and saturation increases with distance from zero. Control inputs are on the top and outputs on the bottom.

### 6.6 Gate

**Shape:** 0–16 channels.  
**Native representation:** boolean off/on.

Multiple Gate sources combine with channel-wise OR:

1. The result channel count is the maximum active channel count of all sources.
2. A one-channel source broadcasts to every result channel.
3. A multi-channel source contributes its matching channel; missing channels are off.
4. The result channel is on if any source or manual gate is on.

When disconnected, press-and-hold turns the gate on for the duration of the press. Command/Ctrl-click toggles a latched gate. When connected, manual presses and the saved latch are ORed with incoming gates.

**Wire rendering:** channel width and spacing match Control. Off is gray and on is black. Gate inputs are on the left and outputs on the right.

### 6.7 Input synchronization

Inputs of the same signal type may be connected with an input-sync wire. A connected component of synchronized inputs behaves as one shared input endpoint:

- Output wires connected to any member feed the group once.
- The signal-type merge is performed once for the group.
- The group has one shared input control: Clock manual frequency only while the group is disconnected, Note transposition, Audio gain/default generator, Control baseline/window, or Gate manual state.
- Changing the control beside any member changes the shared control shown beside every member.
- Every member receives the same final signal.
- Clock groups still allow at most one Clock output source.
- The effective allowed range is the intersection of member ranges. A sync connection with an empty range intersection is rejected.
- Removing a sync wire splits the group. Each resulting group receives a copy of the previously shared control state.
- Rack YAML stores input state on every member module. All members of one sync group must contain identical signal-specific input state. Creating or editing a group copies the canonical state to every member atomically; semantic validation rejects divergent saved member states.

Input-sync wires have stable IDs and use the same signal-specific visual style with a distinct double-line treatment.

### 6.8 Merge and input-control order

For every input or sync group, processing order is:

1. Collect output wires in saved order.
2. Merge according to signal type.
3. Apply the shared input control.
4. Enforce the input’s flavor-specific allowed range.
5. Deliver the result to each consuming module.

This order is normative and must be identical in live and offline rendering.

---

## 7. Modules

### 7.1 Built-in and extension modules

Built-in modules are compiled into the engine and may access private application services such as devices, audio files, MIDI hardware, and specialized transports. Adding or substantially changing one normally requires rebuilding the app.

Extension modules are discovered from extension directories. Their DSP is a WASM or trusted native binary and their optional custom UI is a separately packaged React bundle. Extensions use only the public module ABI and capability API.

Both kinds render and behave identically in the Rack wherever their capabilities overlap.

### 7.2 Identity

Every module instance has:

- A stable UUID used by the document and graph.
- A Rack-unique editable name.
- A stable module type ID.
- A saved module type version and ABI version.
- A selected flavor.

Renaming a module never changes its UUID or breaks wires. The host suggests names and rejects duplicates.

### 7.3 Automatic category

The host assigns the first matching category:

1. No Audio inputs and no Audio outputs:
   - Any Note output → **Sequencer**.
   - Else any Clock output → **Clock**.
   - Else any Control or Gate output → **Logic**.
   - Else → **Utility**.
2. No Audio inputs and one or more Audio outputs → **Generator**.
3. One or more Audio inputs and no Audio outputs → **Output**.
4. One Audio input and one Audio output → **Effect**.
5. More than one Audio input and one Audio output → **Mixer**.
6. Any remaining topology → **Utility**.

Categories control picker grouping and visual accent. A manifest cannot override this classification.

### 7.4 Documentation

Every module type must provide:

- Name and concise summary.
- Qualitative description of its sound or behavior.
- Practical uses for an audio engineer.
- Description for every input, output, flavor, and custom state field.
- At least one example patching recipe.
- Deprecation/replacement guidance when applicable.

Documentation is available from the module’s info corner and module picker without opening an external browser.

### 7.5 Flavors

Every module has one or more flavors. The default flavor is named **Vanilla**.

A flavor may define:

- Hard-coded default state for each input.
- A narrower allowed range for an input.
- Default custom state.
- A qualitative name and description.

A flavor cannot change port IDs, port types, module dimensions, or ABI layout. Changing flavor is one undoable operation. Values outside the new flavor’s range are clamped after confirmation.

### 7.6 Panel geometry and ownership

- Rack grid unit: **64 px at 100% zoom**.
- Module width and height are multiples of four grid units: 256 px, 512 px, and so on.
- Module positions are integer grid coordinates in the YAML, not raw pixels.
- Modules cannot overlap.
- The complete outermost one-unit-thick border—exactly 64 px on every side at 100% zoom—is reserved for the host. A module UI must never paint, position controls in, receive ordinary pointer events from, or claim layout space in this border.
- The four 64×64 px corner tiles are system controls:
  - Top-left: drag/select handle.
  - Top-right: delete.
  - Bottom-left: documentation/info.
  - Bottom-right: context menu.
- Every non-corner tile in the outer border is reserved exclusively for host-rendered input/output ports and that port’s host-owned input state where applicable. Empty border tiles remain host-owned spacing and cannot be reclaimed by the module.
- Clock and Control inputs use the top border and outputs use the bottom.
- Note, Gate, and Audio inputs use the left border and outputs use the right.
- A side needs at least one non-corner border tile per declared port plus its two corner tiles. The final dimension is rounded up to a multiple of four units.
- The manifest fixes port ordering on each side.
- The rectangle inset one unit from every side is the **module-owned center**. For a `W × H`-unit module, the custom UI receives exactly `(W − 2) × (H − 2)` units.
- There is no host title bar, header strip, footer, or name overlay. The module renders its editable/display module name inside its center using the host-provided current name, and may use the rest of its center for any schema-valid display or controls.
- The generic/fault/missing-UI fallback also renders the module name and state inside the center; it never adds a title bar.
- Host chrome and module center have separate clipping and hit-test regions. Tests must fail if custom UI pixels or interactive hit targets escape into the reserved border.

### 7.7 Host controls in input border tiles

Each host-owned input tile presents the port plus compact signal-specific state without consuming the module-owned center:

- Clock: manual frequency while disconnected; incoming frequency and phase while connected.
- Note: current notes using the selected Note presentation.
- Audio: compact spectrogram and gain/default-source state.
- Control: baseline/window/current control.
- Gate: lamp and latch state.

The host owns these border controls and their persistence. A custom UI may mirror or edit them only through the host API. When 64×64 px cannot show every secondary value, the tile shows the primary state and opens a host-owned popover; the popover is anchored outside the module center and never becomes module layout.

### 7.8 Bypass, presets, reset, and deprecation

- Any module with Audio input and Audio output must declare typed bypass routes from outputs to inputs.
- Bypass skips module DSP and copies the declared input voices to outputs.
- Bypass is saved Rack state and is undoable.
- A manifest may declare named presets. Applying one changes only fields included by the preset and is one undoable edit. The Rack stores the resulting state, not the preset name.
- Reset restores the selected flavor’s defaults.
- Deprecated modules remain loadable for old Racks but appear only under a Deprecated picker filter with replacement guidance.

### 7.9 Custom state

A module may define scalar parameters plus additional JSON-compatible custom state for sequencer patterns, selected files, and similar behavior. The host owns the canonical persistent state. The manifest declares parameters by stable ID, kind (`number`, `integer`, `boolean`, or `enum`), default, and allowed range/options, and provides JSON Schema for custom state. Rack YAML stores them separately as `state.parameters` and `state.custom`.

- Parameter values and custom state are embedded as ordinary YAML under the module instance.
- State contains no NaN, infinity, functions, binary objects, or host handles.
- A custom UI may modify it only through validated host capabilities.
- Custom state has its own integer `state_version` and backward-loading contract.
- Numeric/boolean parameter changes are delivered to DSP through the bounded `set_param` path.
- Structural custom-state changes are validated and loaded into a prepared replacement module off the audio thread, then swapped at a block boundary.
- DSP phase, filter delay lines, random-generator cursors, and similar ephemeral runtime state are never Rack state and are transferred only during compatible hot reload.

Every host-rendered input menu also allows a Rack-local label override and color override. These are visual metadata only, are saved with the module in Rack YAML, and never alter signal behavior.

---

## 8. Rack canvas and editing

### 8.1 Infinite canvas

- Infinite in every direction, including negative coordinates.
- Dot grid based on 64 px units.
- Trackpad/wheel pans in both axes.
- Command/Ctrl `+`, `-`, and `0` zoom in, zoom out, and reset pan/zoom.
- Zoom range: 5% through 250%.
- The dot grid coarsens at deep zoom so it remains legible.
- Pan, zoom, and current selection are Rack state saved in YAML.

### 8.2 Placement and collision

- Modules snap to the 64 px grid.
- Modules never overlap after an operation commits.
- Dragging into a neighbor initially stops against it.
- Dragging beyond the neighbor’s midpoint moves past it when space exists.
- When passing is blocked, a directly affected neighbor may be provisionally displaced by the minimum grid distance if doing so does not cascade into a third module.
- Provisional displacement reverts if the dragged module leaves that location before release.
- A committed drag, including any surviving neighbor displacement, is one undo step.
- New modules are placed at the visible free grid position nearest the viewport center.
- A module dragged from the picker is placed nearest its drop point.
- The full module must remain visible when the viewport has enough room.

### 8.3 Module picker

Command/Ctrl+M opens a modal picker containing built-ins, discovered extensions, macros, and deprecated items under a separate filter.

Required behavior:

- Search by module name, type ID, category, and description.
- Category filters.
- Actual scaled panel previews, including safe custom UI preview mode.
- Keyboard focus remains in search while Up/Down changes the highlighted result.
- Enter inserts the highlighted result and closes the picker.
- Escape, backdrop click, and close button dismiss it.
- Click inserts near viewport center.
- Drag inserts near the drop point.
- Preview or custom UI failure falls back to the generated panel without breaking the picker.

### 8.4 Selection and clipboard

- Pressing a module selects it.
- Shift/Command/Ctrl-press toggles membership.
- Background drag creates a marquee; additive modifiers preserve prior selection.
- Command/Ctrl+A selects all.
- Group dragging preserves relative positions.
- Copy/paste includes selected modules, custom state, input state, and wires whose endpoints are wholly inside the selection.
- Pasted modules receive new UUIDs and unique names while preserving relative layout.
- Paste places the group near the current pointer when known, otherwise near viewport center, and avoids overlap.
- Backspace/Delete removes the selection after any required destructive confirmation.

### 8.5 Wires

#### 8.5.1 Routing invariants

- Wires are orthogonal horizontal/vertical polylines from the source port anchor to the target port anchor.
- A wire centerline must never enter, cross, or render over any module rectangle, including its host-owned border and module-owned center. The source/target anchor and the short outward normal segment leaving that exact port are the only contact with a module.
- “Run along a module edge” means use a routing lane immediately outside and parallel to the edge with the standard clearance; it never means draw on top of border tiles, ports, corner controls, or center content.
- Modules are closed obstacles expanded by wire clearance. Adjacent/touching modules are treated as one obstacle except at the active source/target anchors.
- Without user waypoints, the selected route has the minimum Manhattan length among all legal orthogonal paths from A to B.
- Equal-length paths use deterministic tie-breakers in this order: less distance in edge-adjacent lanes, fewer bends, less overlap with existing wires, fewer wire crossings, then stable lexicographic grid order. This makes routes prefer open space while still using module edges when that is shortest or necessary.
- The router must not choose a longer path merely to avoid crossing another wire; module avoidance and shortest length are stronger requirements. Crossed wires render a clear bridge/gap at the crossing without changing graph semantics.
- User waypoints are explicit route constraints. The router finds the shortest legal path from A through each waypoint in saved order to B. Removing all waypoints restores the globally shortest automatic route.
- A waypoint may not lie inside the expanded module obstacle. Moving/resizing a module preserves still-valid waypoints and reports/removes invalid ones according to user confirmation; every unconstrained segment reroutes to the shortest legal path.
- Route output is deterministic for identical module rectangles, anchors, waypoints, and wire ordering. Save/load, zoom, pan, and device pixel ratio cannot change its grid path.
- Moving, resizing, adding, deleting, expanding, collapsing, or nesting a module reroutes every affected wire before the visual frame commits, so no transient frame crosses a module.

The normative router is an A*/Dijkstra search over a sparse rectilinear visibility graph formed from endpoint anchors, waypoint coordinates, and obstacle-clearance lines. The primary edge cost is physical path length. Tie-break costs implement the ordering above and must never outweigh one pixel/unit of primary length. The final path removes collinear intermediate points.

#### 8.5.2 Pointer connection gesture

- Pressing the primary pointer button on any input or output arms that endpoint.
- Dragging beyond the platform drag threshold begins a pending wire. The preview follows the pointer, starts with the endpoint’s required outward-normal segment, uses the signal’s live wire style, and obeys the same module-obstacle rules as a committed route.
- A drag may begin on an output and end on an input or begin on an input and end on an output. Direction is normalized to output → input when committed.
- While dragging, compatible opposite-direction targets highlight; invalid targets remain visible and expose their exact type/range/Clock-source/cycle reason on hover/focus.
- Releasing on a legal target validates the complete graph and commits the wire plus final shortest route as one undoable operation.
- Releasing on the background, the starting endpoint, or an invalid target cancels without graph mutation. Escape, pointer cancel, window blur, or loss of capture also cancels.
- A simple click without crossing the drag threshold focuses/selects the endpoint and does not leave a dangling pending wire.
- Dragging from an output adds a connection to the target input under the normal multi-source rules.
- Dragging from an already connected input first chooses which attached wire to repatch when more than one exists, then carries that wire’s source; release commits target change as one undo step. Escape restores the original wire exactly.
- Shift-click removes the most recently added wire on that endpoint; the context menu can remove a specific attached wire.
- Keyboard Connect mode remains behaviorally equivalent and receives the same validation and final router.

#### 8.5.3 Rendering and persistence

- Saved waypoints and wire ordering are Rack state. Automatically generated bend points are derived and are not serialized.
- Wire hit areas are wider than their visible strokes, remain keyboard-focusable, and never cover module hit targets.
- Wire color, width, banding, dash, and animation are derived only from signal type and live signal state; users cannot override them.
- Crossings, selected/focused state, fault state, pending previews, and reduced-motion behavior are visually distinct without relying only on color.
- At deep zoom-out, wires may simplify animation and bands but preserve topology, obstacle avoidance, endpoints, selection, and signal identity.

If two modules are placed directly side by side, the app offers—not silently performs—a best-effort connection preview. Matching priority is stable port ID, then same-type display order. The user confirms the proposed wires as one undoable operation, and every accepted connection is routed by the same shortest-path algorithm.

### 8.6 Context menus

Rack background:

- Add Module.
- Paste.
- New Rack.
- Open Rack.
- Save.
- Save As.
- View/Edit YAML.

Module or multi-selection:

- Copy.
- Delete.
- Rename single module.
- Change flavor.
- Presets.
- Bypass when supported.
- Reset to flavor defaults.
- Documentation.
- Create Macro.
- Macro-instance actions when applicable.

### 8.7 Undo and redo

- Command/Ctrl+Z undo.
- Command/Ctrl+Shift+Z and Command/Ctrl+Y redo.
- At least 200 committed edits per open Rack.
- Continuous pointer or keyboard adjustment is coalesced until release/commit.
- A drag with collision displacement is one edit.
- Applying valid YAML is one edit.
- Loading, New Rack, and opening another file clear history after dirty-state handling.
- Undo/redo restores every affected Rack field, including layout, wire ordering, viewport, flavors, custom state, and macro instance state.

---

## 9. Rack YAML document

### 9.1 One-file rule

Each Rack is exactly one UTF-8 YAML file with the extension `.bitfiddle.yaml`. All persistent user-authored Rack state is in that file, including:

- Rack identity and revision.
- App and format versions.
- Engine/device configuration.
- Viewport and selection.
- Module UUIDs, names, types, versions, flavors, positions, bypass, input state, and custom state.
- Output wires, input-sync wires, wire order, colors, and waypoints.
- Embedded private definitions and current state for macro instances.
- References and settings for audio files.

No Rack state may exist only in local storage or an opaque side database.

Live telemetry, process-global phase, active audio buffers, hover state, open menus, and incomplete gestures are derived runtime state and are not serialized.

Referenced audio files remain external assets. The YAML stores a user-visible path, a content hash when available, and missing-file status. A missing file must not prevent the Rack from opening.

### 9.2 Editing and synchronization

- The user can switch between Rack and YAML views at any time.
- The YAML editor uses the normative JSON Schema for completion and diagnostics.
- Graphical edits update YAML immediately in memory.
- YAML changes are parsed after a short debounce but apply only on explicit Apply or Command/Ctrl+Enter.
- Apply performs syntax validation, JSON Schema validation, semantic validation, cycle detection, and module availability checks before changing the graph.
- Invalid YAML remains in the editor with diagnostics while the last valid graph continues playing.
- Applying valid YAML is atomic and one undo step.
- Switching away with unapplied YAML prompts to Apply, Keep Editing, or Discard Draft.
- YAML formatting is deterministic: two-space indentation, stable map-key order defined by the serializer, LF endings, one terminal newline, and no anchors or aliases.

### 9.3 Save, dirty state, and recovery

- Command/Ctrl+S saves to the current file.
- Command/Ctrl+Shift+S opens Save As.
- Command/Ctrl+N creates a new Rack.
- Command/Ctrl+O opens a Rack.
- New/Open/Quit prompt Save, Discard, or Cancel when dirty.
- A successful save increments `rack.revision` and updates `modified_at`.
- Saves are atomic: write and fsync a sibling temporary file, then rename.
- Autosave occurs after five seconds of edit inactivity and at least every thirty seconds while dirty.
- For a named Rack, autosave writes an atomic recovery copy under `<data-root>/autosave/`, keyed by Rack UUID; it never silently overwrites the user file.
- Untitled Racks autosave the same way.
- On startup, newer recoveries are offered with Preview, Restore, or Discard.
- The app detects external file modification and offers Reload, Keep Current, or Save As; it never overwrites an externally changed file without confirmation.

### 9.4 Versioning and migration

Every serialization includes:

- `format` and integer `format_version`.
- `app_version`.
- Rack UUID and monotonic revision.
- Every module’s type ID, semantic version, ABI, and custom-state version.
- Every embedded macro’s format version, global UUID, and adopted revision.

Rules:

- The app reads all Rack format versions from v2 onward through explicit migration functions.
- Migration is pure, deterministic, and tested with fixtures.
- Opening an older Rack migrates in memory and marks it dirty; the original file is untouched until save.
- A newer unsupported format opens read-only with an actionable error.
- Missing module types render a placeholder preserving all saved state and wires.
- Missing/renamed ports produce non-fatal load warnings and preserved unresolved wire records; they are not silently deleted.
- A module update must accept its prior persisted state versions or declare a host-run pure migration.

### 9.5 Example

```yaml
format: bitfiddle-rack
format_version: 2
app_version: 2.0.0
rack:
  id: 7d44997e-b35a-4c69-a1fd-fab6ce28738d
  name: First Rack
  revision: 3
  created_at: "2026-05-21T18:00:00Z"
  modified_at: "2026-05-21T18:30:00Z"
engine:
  sample_rate: 48000
  block_size: 128
  default_device_id: built-in-output
view:
  pan: { x: 0, y: 0 }
  zoom: 1
  selected: []
modules:
  - id: c33d9f62-57d7-4cc1-b6ad-69513dc91008
    name: Main Oscillator
    type_id: app.oscillator
    type_version: 2.0.0
    abi: builtin-2
    state_version: 1
    flavor: Vanilla
    position: { x: 0, y: 0 }
    bypassed: false
    input_ui: {}
    inputs:
      note:
        signal: note
        manual_hz: 440
        transpose_semitones: 0
    state:
      parameters:
        waveform: sine
      custom: {}
wires: []
input_sync: []
macros: []
```

### 9.6 Normative Rack JSON Schema

The implementation must install this schema as `schemas/rack.schema.json`, use JSON Schema draft 2020-12, and validate YAML after parsing it into the JSON data model. Semantic constraints noted after the schema are additionally required because JSON Schema cannot express graph references, uniqueness across arrays, or cycle freedom.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://bitfiddle.local/schemas/rack-2.schema.json",
  "title": "bitfiddle Rack",
  "type": "object",
  "additionalProperties": false,
  "required": [
    "format",
    "format_version",
    "app_version",
    "rack",
    "engine",
    "view",
    "modules",
    "wires",
    "input_sync",
    "macros"
  ],
  "properties": {
    "format": { "const": "bitfiddle-rack" },
    "format_version": { "const": 2 },
    "app_version": { "$ref": "#/$defs/semver" },
    "rack": { "$ref": "#/$defs/rackMetadata" },
    "engine": { "$ref": "#/$defs/engine" },
    "view": { "$ref": "#/$defs/view" },
    "modules": {
      "type": "array",
      "items": { "$ref": "#/$defs/module" }
    },
    "wires": {
      "type": "array",
      "items": { "$ref": "#/$defs/wire" }
    },
    "input_sync": {
      "type": "array",
      "items": { "$ref": "#/$defs/inputSync" }
    },
    "macros": {
      "type": "array",
      "items": { "$ref": "#/$defs/macroInstance" }
    }
  },
  "$defs": {
    "uuid": {
      "type": "string",
      "format": "uuid"
    },
    "semver": {
      "type": "string",
      "pattern": "^(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)(?:-[0-9A-Za-z.-]+)?(?:\\+[0-9A-Za-z.-]+)?$"
    },
    "identifier": {
      "type": "string",
      "pattern": "^[A-Za-z][A-Za-z0-9_.-]{0,127}$"
    },
    "signal": {
      "enum": ["clock", "note", "audio", "control", "gate"]
    },
    "gridPoint": {
      "type": "object",
      "additionalProperties": false,
      "required": ["x", "y"],
      "properties": {
        "x": { "type": "integer" },
        "y": { "type": "integer" }
      }
    },
    "rackMetadata": {
      "type": "object",
      "additionalProperties": false,
      "required": ["id", "name", "revision", "created_at", "modified_at"],
      "properties": {
        "id": { "$ref": "#/$defs/uuid" },
        "name": { "type": "string", "minLength": 1, "maxLength": 200 },
        "revision": { "type": "integer", "minimum": 0 },
        "created_at": { "type": "string", "format": "date-time" },
        "modified_at": { "type": "string", "format": "date-time" }
      }
    },
    "engine": {
      "type": "object",
      "additionalProperties": false,
      "required": ["sample_rate", "block_size", "default_device_id"],
      "properties": {
        "sample_rate": { "type": "integer", "minimum": 8000, "maximum": 384000 },
        "block_size": { "type": "integer", "minimum": 16, "maximum": 4096 },
        "default_device_id": { "type": ["string", "null"] }
      }
    },
    "view": {
      "type": "object",
      "additionalProperties": false,
      "required": ["pan", "zoom", "selected"],
      "properties": {
        "pan": {
          "type": "object",
          "additionalProperties": false,
          "required": ["x", "y"],
          "properties": {
            "x": { "type": "number" },
            "y": { "type": "number" }
          }
        },
        "zoom": { "type": "number", "minimum": 0.05, "maximum": 2.5 },
        "selected": {
          "type": "array",
          "items": { "$ref": "#/$defs/uuid" },
          "uniqueItems": true
        }
      }
    },
    "module": {
      "type": "object",
      "additionalProperties": false,
      "required": [
        "id",
        "name",
        "type_id",
        "type_version",
        "abi",
        "state_version",
        "flavor",
        "position",
        "bypassed",
        "input_ui",
        "inputs",
        "state"
      ],
      "properties": {
        "id": { "$ref": "#/$defs/uuid" },
        "name": { "type": "string", "minLength": 1, "maxLength": 120 },
        "type_id": { "$ref": "#/$defs/identifier" },
        "type_version": { "$ref": "#/$defs/semver" },
        "abi": { "enum": ["builtin-2", "wasm-2", "native-2", "missing-2"] },
        "state_version": { "type": "integer", "minimum": 1 },
        "flavor": { "type": "string", "minLength": 1, "maxLength": 80 },
        "position": { "$ref": "#/$defs/gridPoint" },
        "bypassed": { "type": "boolean" },
        "input_ui": {
          "type": "object",
          "additionalProperties": { "$ref": "#/$defs/inputUi" }
        },
        "inputs": {
          "type": "object",
          "additionalProperties": { "$ref": "#/$defs/inputState" }
        },
        "state": { "$ref": "#/$defs/moduleState" }
      }
    },
    "moduleState": {
      "type": "object",
      "additionalProperties": false,
      "required": ["parameters", "custom"],
      "properties": {
        "parameters": { "type": "object" },
        "custom": { "type": "object" }
      }
    },
    "inputUi": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "label": { "type": "string", "minLength": 1, "maxLength": 80 },
        "color": { "type": "string", "pattern": "^#[0-9A-Fa-f]{6}$" }
      }
    },
    "inputState": {
      "oneOf": [
        { "$ref": "#/$defs/clockInput" },
        { "$ref": "#/$defs/noteInput" },
        { "$ref": "#/$defs/audioInput" },
        { "$ref": "#/$defs/controlInput" },
        { "$ref": "#/$defs/gateInput" }
      ]
    },
    "clockInput": {
      "type": "object",
      "additionalProperties": false,
      "required": ["signal", "manual_hz"],
      "properties": {
        "signal": { "const": "clock" },
        "manual_hz": { "type": "number", "minimum": 0, "maximum": 40 }
      }
    },
    "noteInput": {
      "type": "object",
      "additionalProperties": false,
      "required": ["signal", "manual_hz", "transpose_semitones"],
      "properties": {
        "signal": { "const": "note" },
        "manual_hz": { "type": "number", "minimum": 20, "maximum": 20000 },
        "transpose_semitones": { "type": "number", "minimum": -240, "maximum": 240 }
      }
    },
    "audioInput": {
      "type": "object",
      "additionalProperties": false,
      "required": ["signal", "gain", "default_source", "seed"],
      "properties": {
        "signal": { "const": "audio" },
        "gain": { "type": "number", "minimum": 0, "maximum": 4 },
        "default_source": {
          "enum": ["silence", "white_noise", "sine_440", "saw_440", "triangle_440", "square_440"]
        },
        "seed": { "type": "integer", "minimum": 0 }
      }
    },
    "controlInput": {
      "type": "object",
      "additionalProperties": false,
      "required": ["signal", "baseline", "window"],
      "properties": {
        "signal": { "const": "control" },
        "baseline": { "type": "number", "minimum": -1, "maximum": 1 },
        "window": { "type": "number", "minimum": -2, "maximum": 2 }
      }
    },
    "gateInput": {
      "type": "object",
      "additionalProperties": false,
      "required": ["signal", "latched"],
      "properties": {
        "signal": { "const": "gate" },
        "latched": { "type": "boolean" }
      }
    },
    "outputEndpoint": {
      "type": "object",
      "additionalProperties": false,
      "required": ["module", "port"],
      "properties": {
        "module": { "$ref": "#/$defs/uuid" },
        "port": { "$ref": "#/$defs/identifier" }
      }
    },
    "inputEndpoint": {
      "type": "object",
      "additionalProperties": false,
      "required": ["module", "port"],
      "properties": {
        "module": { "$ref": "#/$defs/uuid" },
        "port": { "$ref": "#/$defs/identifier" }
      }
    },
    "wire": {
      "type": "object",
      "additionalProperties": false,
      "required": ["id", "signal", "source", "target", "order", "waypoints"],
      "properties": {
        "id": { "$ref": "#/$defs/uuid" },
        "signal": { "$ref": "#/$defs/signal" },
        "source": { "$ref": "#/$defs/outputEndpoint" },
        "target": { "$ref": "#/$defs/inputEndpoint" },
        "order": { "type": "integer", "minimum": 0 },
        "waypoints": {
          "type": "array",
          "items": { "$ref": "#/$defs/gridPoint" }
        }
      }
    },
    "inputSync": {
      "type": "object",
      "additionalProperties": false,
      "required": ["id", "signal", "a", "b", "waypoints"],
      "properties": {
        "id": { "$ref": "#/$defs/uuid" },
        "signal": { "$ref": "#/$defs/signal" },
        "a": { "$ref": "#/$defs/inputEndpoint" },
        "b": { "$ref": "#/$defs/inputEndpoint" },
        "waypoints": {
          "type": "array",
          "items": { "$ref": "#/$defs/gridPoint" }
        }
      }
    },
    "macroInstance": {
      "type": "object",
      "additionalProperties": false,
      "required": [
        "module_id",
        "global_id",
        "global_name",
        "format_version",
        "adopted_revision",
        "adopted_definition",
        "current_definition"
      ],
      "properties": {
        "module_id": { "$ref": "#/$defs/uuid" },
        "global_id": { "$ref": "#/$defs/uuid" },
        "global_name": { "type": "string", "minLength": 1 },
        "format_version": { "const": 2 },
        "adopted_revision": { "type": "integer", "minimum": 1 },
        "adopted_definition": { "type": "object" },
        "current_definition": { "type": ["object", "null"] }
      }
    }
  }
}
```

Additional semantic validation must enforce:

- Unique module, wire, sync-wire, and module-name identities.
- All endpoint module and port references exist or are retained as explicitly unresolved load warnings.
- Saved signal types match current port declarations.
- Module positions and dimensions do not overlap.
- `view.selected` refers only to current module UUIDs.
- Exactly one module record corresponds to each macro instance `module_id`.
- Nested macros recursively expand to valid modules and wires without a self-reference or dependency cycle.
- No output graph cycle exists after input-sync groups are collapsed.
- Clock input groups have no more than one output source.
- Input-sync range intersections are non-empty.
- Module custom state validates against that module version’s state schema.

---

## 10. Macros

### 10.1 Definition and storage

A macro is a reusable global module type made from a Rack subgraph. It may contain ordinary modules and other macros. It is not owned by or stored only in a particular Rack.

Each global macro is one YAML file under `<data-root>/macros/<uuid>.bitfiddle-macro.yaml` and has:

- Stable UUID.
- Name and description.
- Macro format version 2.
- Monotonic revision.
- Creation and modification timestamps.
- Internal ordinary modules and nested macro instances, including positions, input state, parameters, custom state, and private adopted definitions.
- Internal wires and input-sync groups.
- External interface mapping with stable input/output IDs, types, order, descriptions, and internal wall connections.
- Preview layout and expanded bounding-box geometry.

The app must install and use a JSON Schema for macro files. Macro files and Rack-embedded private macro definitions use the same recursive definition shape. Every persistent nested definition is YAML/JSON data; no implementation closure, host handle, or opaque graph blob is allowed.

### 10.2 Expanded editor and inner walls

An expanded macro renders its internal graph inside one host-drawn bounding box. The box has two related surfaces:

- **Outer wall:** the macro-level ports seen by the containing Rack or containing macro.
- **Inner wall:** editable boundary targets seen while the macro is expanded.

The inner wall is the primary interface-authoring UI. A user creates macro-level ports by wiring internal modules to it:

- Drag from an internal **output** to a compatible inner-wall target to create a macro **output**.
- Drag from an internal **input** to a compatible inner-wall target to create a macro **input**; the direction is displayed as outer source → wall → internal input.
- Drag another compatible internal input to an existing macro-input wall port to fan that macro input out to several internal inputs.
- One macro output maps to one internal output. If several internal signals must become one macro output, they must be combined explicitly by a typed module before reaching the wall.
- Clock and Control wall ports occupy top/bottom according to the normal input/output placement law. Note, Gate, and Audio wall ports occupy left/right.
- Invalid signal types, empty flavor-range intersections, Clock-source violations, graph cycles, or recursive macro cycles are rejected before mutation.

Creating a wall connection performs one undoable structural edit:

1. Allocate a stable interface port ID.
2. Derive a unique human-readable name from the internal port; no alias is created.
3. Create the persisted wall mapping.
4. Render the corresponding outer port immediately.
5. Preserve any external wire by stable interface port ID across later rename/reorder edits.

The wall-port inspector allows the user to rename a port, edit its description, reorder it on its legal side, and review every internal endpoint it feeds. It does not allow changing signal type in place. A type change is remove-and-create so external wires cannot silently reinterpret data.

Removing the last inner connection removes the macro-level port after confirmation if any outer wire uses it. Removing one of several destinations from a macro input keeps the macro-level port. Deleting a connected macro-level port either removes the confirmed outer wires in the same undo step or preserves them as explicit unresolved connections according to the operation the user selects.

Every inner-wall action has keyboard parity:

- Connect mode visits compatible wall targets after internal ports.
- `:macro expose input <module>.<port> as <name>` creates or extends a macro input.
- `:macro expose output <module>.<port> as <name>` creates a macro output.
- `:macro unexpose <port>` removes a wall port with the same confirmation rules.
- Arrow keys navigate wall ports; Enter edits; bracket commands reorder. Lowercase `k` enters Keyboard mode instead.

Macro parameters may also be promoted to the collapsed outer face through the wall-port inspector. A promoted control stores a stable control ID, human-readable name/description, widget kind, allowed range/options, and an internal target path made from nested instance UUIDs plus the final parameter ID. Several compatible internal parameters may share one promoted control after the user confirms their range intersection. Promoted controls are not signal ports and cannot be wired.

The macro definition stores a collapsed-face layout made from promoted controls, labels, meters, lamps, and bounded telemetry views. A stock macro may additionally provide a sandboxed custom UI bundle, but its persistent state remains the recursive macro YAML and every custom interaction must map to promoted controls or explicit host capabilities. Expanding the macro always reveals the ordinary modules and nested macros that implement it.

### 10.3 Creating a macro

1. Select one or more ordinary modules and/or macro instances in a Rack or expanded macro.
2. Invoke **Create Macro** from the keyboard command or context menu.
3. Open the new macro expanded around the selected content.
4. Preserve every boundary connection as a proposed wall connection:
   - An incoming wire proposes a macro input wired from the inner wall to the internal destination.
   - An outgoing wire proposes a macro output wired from the internal source to the inner wall.
5. Show the proposed interface on both inner and outer walls while leaving it editable.
6. The user may wire other compatible internal ports to empty/existing inner-wall targets, remove proposed wall connections, rename ports, order ports, and write descriptions.
7. The user supplies a global macro name and description.
8. Validation rejects duplicate names, invalid interfaces, graph cycles, recursive macro dependencies, and expansion-budget violations.
9. Commit creates a new global macro UUID at revision 1 and replaces the selected Rack subgraph with one macro instance as a single undoable operation.

Creating a macro from a Rack or parent macro does not make that document its owner. Deleting or renaming the parent does not affect the global macro.

### 10.4 Nested macros

Macros may contain macros to any useful composition depth within explicit safety budgets.

- A nested macro instance uses the same private adopted/current-definition model as a top-level Rack instance.
- A Rack or parent macro may contain any number of sibling instances of the same child macro.
- A macro definition may not directly or indirectly contain itself. Any dependency path from a definition back to the same definition UUID is rejected.
- The global macro dependency graph must be acyclic.
- Validation follows both global references and embedded private definitions and reports the complete dependency path for a cycle, such as `Drum Rack → Voice → Drum Rack`.
- Runtime compilation recursively expands nested definitions before graph topological sorting, latency analysis, buffer planning, and cycle detection.
- Expanded runtime instance IDs derive deterministically from the full parent-instance UUID path plus internal UUID, so sibling instances never share state.
- External interface mappings are resolved at each boundary. Signal type, polyphony order, merge order, input-sync semantics, and bypass remain identical to an equivalent manually expanded Rack.
- A nested macro cannot use its parent’s private modules, wires, or state except through the parent/child wall ports.

Safety budgets are configuration constants covered by tests:

- Maximum nested depth: 32.
- Maximum recursively expanded modules per top-level Rack: 10,000.
- Maximum recursively expanded wires plus sync links: 50,000.

Exceeding a budget produces a load/editor diagnostic and leaves the last valid graph running; it never partially expands or crashes.

### 10.5 Macro instances and recursive self-containment

When inserted, a macro instance embeds a private copy of the global definition and adopted revision in the containing Rack or macro YAML. This copy includes the complete transitive definitions of nested instances needed to execute it. Therefore:

- A Rack always plays the full recursive definition tree it was saved with.
- A global macro edit never silently changes existing Racks or parent macros.
- Two instances, including nested instances, can diverge independently.
- A Rack remains usable if any global macro file in its dependency tree is deleted or unavailable.
- Save/load never depends on resolving a global file merely to reconstruct sound.

An instance can be expanded in place. Its direct internal modules and direct nested macro instances render inside its bounding box. A nested instance can be expanded inside that box, producing another visible boundary. Breadcrumbs and tree navigation identify the current editing depth. The outermost box remains selected, moved, copied, pasted, and deleted as one top-level object unless the user explicitly enters it.

### 10.6 Updating a global macro

An expanded instance offers **Publish as New Revision**:

1. Validate its current private recursive definition and wall interface.
2. Show a structural diff against the current global revision, including nested-definition and interface changes.
3. Require confirmation.
4. Atomically replace the global macro file with revision + 1.
5. Update that publishing instance’s adopted definition and adopted revision to the published state.
6. Leave every other instance in every Rack and parent macro unchanged.

Publishing a parent macro captures the nested definitions currently embedded in that parent. It does not implicitly pull the latest global revision of any nested macro. The editor must show nested instances with newer global revisions and offer explicit per-instance or selected/bulk **Pull Latest** before publishing the parent.

A dedicated macro editor may perform the same revisioned update without a Rack owner; it edits the global object directly and uses the same validation and atomic save rules.

### 10.7 Synchronizing and breaking an instance

- **Pull Latest:** show a recursive diff and confirmation, then replace the instance’s private definition with the latest global revision. Local instance edits, including nested adopted revisions, are discarded.
- **Reset Instance:** restore the full private definition originally adopted by this instance, not the current global revision.
- **Break Macro:** replace one macro boundary with its direct internal ordinary modules and direct nested macro instances at their current positions; nested macros remain macros.
- **Flatten Recursively:** replace the instance and every nested macro with ordinary modules/wires after showing expanded counts and requiring confirmation.
- **Save as New Macro:** publish the current recursive instance as a new global UUID at revision 1.

Pulling a nested macro updates only that nested private instance and marks its containing parent definition dirty. Pulling a parent replaces the whole parent tree as stated above. Each operation is one undoable document edit. Publishing a global file is not undone by Rack Undo; the confirmation must say so.

### 10.8 Renaming and deleting a global macro

- Rename changes global metadata but preserves UUID and increments revision.
- Existing Rack and parent-macro instances keep their saved display metadata until Pull Latest; the picker shows the current global name.
- Delete requires confirmation and lists currently open Rack and macro instances when known.
- Deleting removes the global file and picker entry but never changes saved private instances at any depth.
- An orphaned nested or top-level instance can still expand, break, flatten, reset, or Save as New Macro, but cannot Pull Latest.
- Deleting a macro is irreversible from Rack Undo; the app keeps the previous file in the operating system trash when supported.

### 10.9 Migration from macro format 1

Macro format 1 definitions contain no nested instances and use an explicit interface mapping. Migration to format 2 is deterministic:

- Preserve global UUID, revision, module/wire UUIDs, state, and interface port IDs.
- Convert each interface mapping into the equivalent persisted inner-wall connection.
- Leave the internal graph and sound unchanged.
- Embed the converted definition when an old Rack is opened; do not overwrite the source Rack/global file until explicit Save.

---

## 11. Extension system

### 11.1 Package layout

An extension is a directory or installable `.bitfiddle-ext` ZIP with this layout:

```text
my-extension/
  manifest.json
  dsp.wasm              # abi = wasm-2
  # or one of:
  dsp.dylib
  dsp.so
  dsp.dll               # abi = native-2
  ui.js                 # optional custom React UI bundle
  README.md
```

ZIP installation extracts into a content-addressed staging directory, validates everything, and atomically moves the extension under `<data-root>/extensions/`. Directory traversal, symlinks escaping the package, duplicate IDs, oversized files, and unsupported ABI versions are rejected.

### 11.2 Manifest

Required conceptual fields:

```json
{
  "id": "com.example.delay",
  "name": "Delay",
  "version": "2.1.0",
  "abi": "wasm-2",
  "description": "...",
  "deprecated": false,
  "latency_samples": 0,
  "size": { "width_units": 8, "height_units": 8 },
  "state_version": 1,
  "custom_state_schema": { "type": "object" },
  "parameters": [
    {
      "id": "feedback",
      "name": "Feedback",
      "kind": "number",
      "default": 0.25,
      "minimum": 0,
      "maximum": 0.95
    }
  ],
  "inputs": [
    {
      "id": "audio_in",
      "name": "Audio In",
      "signal": "audio",
      "description": "...",
      "order": 0
    }
  ],
  "outputs": [
    {
      "id": "audio_out",
      "name": "Audio Out",
      "signal": "audio",
      "description": "...",
      "order": 0
    }
  ],
  "flavors": [
    {
      "name": "Vanilla",
      "description": "...",
      "inputs": {},
      "state": {}
    }
  ],
  "bypass": { "audio_out": "audio_in" },
  "presets": [],
  "ui": { "entry": "ui.js", "api": "ui-2" }
}
```

The implementation must define and ship a normative JSON Schema for `manifest.json`. It must reject unknown fields, duplicate IDs, unsupported signal types, invalid parameter declarations, dimensions not divisible by four units, invalid bypass routes, custom state without a schema, and port counts that exceed ABI limits. Parameters are module-owned state and are not ports: they cannot be wired, merged, or synchronized.

There are no generic unit or conversion fields.

### 11.3 DSP buffer layout

`wasm-2` and `native-2` share one conceptual ABI and the official Rust SDK must hide raw details.

Each logical port is allocated in manifest order. Data is planar with a fixed `max_frames` stride:

- Clock: 1 lane.
- Note: 16 lanes.
- Control: 16 lanes.
- Gate: 16 lanes encoded as `0.0` or `1.0` at each sample.
- Audio: 32 lanes ordered left/right per voice.

Port lane offsets are the prefix sum of those fixed lane counts. A sample is located at:

```text
base + (port_lane_offset + lane) * max_frames + frame
```

Each port also has one `u32` active-channel count. Audio counts voices, not lanes. Clock is 0 or 1. Every Audio port additionally has one `u16` mono mask; bit N is set when voice N uses its left lane as mono and its right lane must be ignored/copied by the consumer. Bits at or above the active voice count must be zero. A module must write every active output sample, active-channel count, and Audio mono mask on every process call. The host clears outputs when a module faults.

### 11.4 WASM ABI `wasm-2`

A WASM module has no required imports and exports linear memory plus:

```text
bitfiddle_v2_abi_version() -> u32                         # must return 2
bitfiddle_v2_init(sample_rate: f32, max_frames: u32) -> i32
bitfiddle_v2_input_ptr() -> u32
bitfiddle_v2_output_ptr() -> u32
bitfiddle_v2_input_channels_ptr() -> u32
bitfiddle_v2_output_channels_ptr() -> u32
bitfiddle_v2_input_audio_mono_masks_ptr() -> u32
bitfiddle_v2_output_audio_mono_masks_ptr() -> u32
bitfiddle_v2_document_state_ptr() -> u32
bitfiddle_v2_document_state_capacity() -> u32
bitfiddle_v2_runtime_state_ptr() -> u32
bitfiddle_v2_runtime_state_capacity() -> u32
bitfiddle_v2_set_param(index: u32, value: f32) -> i32
bitfiddle_v2_load_document_state(state_version: u32, length: u32) -> i32
bitfiddle_v2_process(
  frames: u32,
  connected_mask_low: u64,
  connected_mask_high: u64,
  absolute_sample_time: u64
) -> i32
bitfiddle_v2_save_runtime_state() -> u32                 # UTF-8 JSON byte length
bitfiddle_v2_load_runtime_state(runtime_version: u32, length: u32) -> i32
bitfiddle_v2_reset() -> i32
```

Requirements:

- Maximum 128 total input ports and 128 total output ports.
- `connected_mask_*` marks logical input ports with at least one output source or input-sync source.
- Return `0` for success; nonzero values are module-defined errors surfaced by the host.
- Document-state and runtime-state capacities are each at least 64 KiB and at most 1 MiB.
- `load_document_state` receives the canonical UTF-8 JSON object `{parameters, custom}` after host validation and migration. It is used during off-thread construction, not for ordinary scalar automation.
- `set_param` receives manifest parameter index and a scalar encoded as `f32`; boolean is `0` or `1`, integer is integral, and enum is its zero-based option index. It may run on the audio thread and must be real-time safe.
- `save_runtime_state` and `load_runtime_state` transfer a UTF-8 JSON object containing only ephemeral DSP runtime state during hot reload. This state is never written to Rack YAML.
- A new extension version must accept the immediately prior runtime version or explicitly declare that hot reload requires a reset; a failed required transfer leaves old instances running.
- The host validates all pointers, sizes, active-channel counts, Audio mono masks, booleans, finite values, and state JSON before use.
- Wasmtime fuel and epoch interruption bound each process call to the block deadline plus configured headroom.
- WASM has no filesystem, network, environment, clock, random, or Tauri access. Deterministic services are supplied only through explicit future ABI versions.
- A trap, timeout, invalid pointer, or invalid output silences that instance for the block, increments its fault count, and posts a non-blocking diagnostic. Repeated faults disable the instance until reset.

### 11.5 Native ABI `native-2`

Native modules expose a versioned C vtable through the symbol `bitfiddle_native_v2_entry`. The vtable has:

- ABI version and struct size.
- `create`, `destroy`, `process`, `set_param`, `load_document_state`, `save_runtime_state`, `load_runtime_state`, and `reset` functions.
- The same planar buffers, active-channel arrays, Audio mono masks, process arguments, return codes, and document/runtime JSON state contracts as WASM.

Native modules are unsandboxed and execute arbitrary code with the app’s privileges. Installation and first load must show that trust warning. Real-time restrictions are contractual and checked by conformance tests but cannot be enforced against malicious native code.

The host loads each native artifact from a unique content-hashed path so an updated library can coexist with old live instances until a block-boundary swap completes.

### 11.6 Hot reload

The control thread watches extension directories with debouncing and content hashes.

On DSP change:

1. Compile/load and validate the candidate off the audio thread.
2. Instantiate a candidate for every live instance.
3. Read canonical host-owned document state and save each old instance’s ephemeral runtime state.
4. Load document state and then compatible runtime state into each candidate.
5. Process at least two silent validation blocks and verify finite, in-range metadata.
6. If any step fails, keep all old instances running and report the error.
7. If all steps pass, publish all replacements as one immutable graph edit.
8. Swap at one block boundary.
9. Return old instances to the off-thread garbage queue.

Wiring, UUIDs, names, input state, macro state, and Rack dirty state are unchanged. The target is no xrun and no interruption longer than one block.

On manifest port changes, automatic live swap is allowed only when every currently used port ID retains its signal type. Otherwise the candidate is staged and the user receives a migration preview.

On UI change, remount the sandboxed UI in preview first, then swap it while keeping all canonical host state. UI reload never touches DSP state.

### 11.7 Custom React UI packaging and sandbox

Built-in custom UIs may render directly in the host React tree only inside the clipped module-owned center. Extension `ui.js` bundles run in a sandboxed iframe whose viewport is exactly the module-owned center; the host never mounts custom UI over the one-unit border.

The iframe:

- Uses `sandbox="allow-scripts"` without `allow-same-origin`.
- Has a host-controlled CSP: no network, no navigation, no forms, no popups, no eval, and no access to Tauri globals.
- Is created from host-generated `srcdoc`; the host injects the validated extension bundle plus pinned React and `@bitfiddle/ui-sdk` bytes as local Blob URLs through an inline import map before execution. No resource is fetched by the iframe. Blob URLs are revoked when the UI unmounts.
- Communicates only through a versioned `postMessage` capability protocol.
- Receives no filesystem, device, clipboard, camera, microphone, MIDI, or network capability.
- Cannot use local storage, IndexedDB, or cookies.

The `ui-2` capability protocol provides:

- Initial immutable manifest, host-provided current module name, exact center dimensions, current parameters, custom state, and input state.
- `setParameter(id, value)` validated against the manifest declaration.
- `setCustomState(patch)` validated against the custom-state schema.
- Signal-specific setters for host-owned input controls.
- `beginEdit`, `commitEdit`, and `cancelEdit` for undo coalescing.
- Read-only telemetry subscriptions capped at 30 Hz.
- Optional bounded raw capture only for ports whose manifest explicitly declares `capture: true`.
- Preview mode with inert setters and synthetic silence.

Every message includes iframe instance ID, monotonically increasing sequence number, and schema-validated payload. Unknown or oversized messages are rejected. A UI crash, rejection, timeout, or render loop replaces only that center with a generated-panel fallback and Retry button; DSP continues.

Extension UIs must treat host state as canonical. Any iframe-local state is ephemeral and must be reconstructible after remount.

---

## 12. Keyboard system

### 12.1 Principles

- Every Rack operation available by mouse is available by keyboard.
- Arrow keys are the canonical spatial navigation and adjustment keys. Lowercase `k` is globally reserved for Keyboard mode and is never a movement/adjustment alias.
- Current mode and available keys are visible.
- Every focused module, input, output, and wire exposes its full name and available shortcut hints on screen.
- Commands resolve Rack-unique module names plus stable port IDs, never DOM order.
- Text fields use ordinary text editing and do not trigger Rack commands; typing `k` in a text field inserts text rather than changing mode.
- QWERTY musical-keyboard modules receive keyboard events only while explicitly in Keyboard mode.

### 12.2 Modes

1. **Normal mode** — default Rack navigation and commands.
2. **Visual mode** — build/toggle a multi-selection.
3. **Move mode** — move selected modules on the grid.
4. **Connect mode** — choose source/sync endpoint and destination.
5. **Adjust mode** — adjust one input using signal-specific behavior.
6. **Command mode** — enter `:` commands.
7. **Text mode** — ordinary editing in YAML, search, names, and custom text fields.
8. **Keyboard mode (`k` mode)** — every present QWERTY Input module receives application keyboard events.

Escape moves one level toward Normal mode and never commits a partial edit, except that Keyboard mode has the stronger exclusive-exit rules in §12.5.

### 12.3 Normal-mode keys

- Arrows: focus the nearest module in that direction.
- `k`: enter Keyboard mode.
- `Enter`: enter the focused module; arrows then move among its ports and controls.
- `Tab` / Shift+Tab: next/previous actionable element.
- `Space`: select focused module, replacing selection.
- `v`: enter Visual mode; Space toggles focused module.
- `m`: enter Move mode for selection.
- `c`: enter Connect mode from focused port.
- `a`: open module picker.
- `d`: delete selection with confirmation when needed.
- `y`: copy selection.
- `p`: paste.
- `u`: undo.
- Ctrl+R: redo.
- `:`: Command mode.
- `/`: open module picker with search focused.
- `?`: context help and shortcut legend.

Platform shortcuts in this PRD remain available in every non-text mode except Keyboard mode, where all keys other than Escape are musical input.

### 12.4 Move, connect, and adjust modes

**Move mode**

- Arrows move by one 64 px grid unit.
- Shift moves by four units.
- Enter commits the entire move as one undo step.
- Escape restores original positions.
- Collision behavior matches pointer dragging.

**Connect mode**

- The starting endpoint is announced and highlighted.
- Navigation visits only legal compatible endpoints by default; `!` reveals invalid endpoints and reasons.
- Enter completes the connection.
- For a multi-source input, completion adds a wire.
- For Clock, an already sourced input is invalid.
- Starting on an input and choosing another input creates an input-sync wire.
- Escape cancels without graph mutation.

**Adjust mode**

- Up increases and Down decreases by the standard quantized step.
- Right increases and Left decreases by a fine step.
- Shift applies a coarse step.
- `0` resets to flavor default.
- Signal-specific secondary controls are selected with Tab: Clock manual frequency when disconnected; Note transposition; Audio gain/source; Control baseline/window; Gate momentary/latch.
- Enter commits one undo step; Escape restores the pre-adjust value.

### 12.5 Keyboard mode (`k` mode)

- Pressing unmodified lowercase `k` from any non-text Rack state where no blocking dialog is open enters Keyboard mode. If a transient Move/Connect/Adjust operation is active, entering first cancels that operation without committing it. The entry `k` keydown/key-up pair is not forwarded as a musical event.
- While Keyboard mode is active, every application `keydown` and `keyup` event except Escape is delivered in order to **every QWERTY Input module present in the active Rack**, including QWERTY modules inside expanded or collapsed nested macros.
- Delivery is broadcast, not focus-based: a QWERTY module does not need selection or focus, and several modules receive the same normalized event independently.
- The normalized event includes physical key `code`, logical `key`, down/up, repeat, Shift/Control/Option/Command state, and a monotonic sequence number. Mappings choose physical or logical semantics explicitly in module state.
- Key repeat is forwarded rather than synthesized. Each QWERTY module decides whether a mapping retriggers on repeat.
- Except for Escape, keys that are normally Rack or platform shortcuts are consumed as Keyboard-mode input and do not trigger navigation, edit, save, command, or module-picker actions.
- Escape is the **only** user action that exits Keyboard mode. Clicking the background, modules, mode indicator, another page, or another Rack does not exit. No other key, chord, timeout, focus change, or completed pointer action exits.
- Escape is consumed by the host, is not forwarded to QWERTY modules, broadcasts all-notes/all-gates-off, clears held-key state, and returns directly to Normal mode.
- On app/window blur, suspend, or device loss, the host broadcasts all-notes/all-gates-off to prevent stuck notes but remains in Keyboard mode. Event delivery resumes on focus. Only a later Escape exits.
- Opening/switching a Rack by pointer while Keyboard mode remains active atomically releases the old Rack’s QWERTY modules and begins delivering subsequent events to all QWERTY modules in the new active Rack.
- If no QWERTY module is present, Keyboard mode still activates visibly and consumes keys. The mode indicator says **No QWERTY Input modules**; adding/opening one makes it a recipient immediately.
- Keyboard mode is ephemeral UI state and is not stored in Rack YAML. A new app process starts in Normal mode.
- The mode indicator is persistent, high-contrast, screen-reader announced, names the recipient count, and says **Escape exits**. It is not an exit button.

### 12.6 Command language

Command mode opens a bottom command line. It supports history, completion, and inline validation. Commands use readable names only; there is no alias or compact-token grammar.

Readable commands:

```text
:add <type-or-macro> [as <name>] [at <x>,<y>]
:wire <module>.<output> -> <module>.<input>
:sync <module>.<input> <-> <module>.<input>
:unwire <wire-or-endpoint>
:select <module>...
:set <module>.<input> <signal-specific-value>
:move <module-or-selection> <x> <y>
:rename <module> <name>
:flavor <module> <flavor>
:bypass <module> on|off
:delete <module-or-selection>
:copy <module-or-selection>
:paste [at <x>,<y>]
:macro create <name>
:macro expose input <module>.<port> as <name>
:macro expose output <module>.<port> as <name>
:macro unexpose <port>
:macro publish <instance>
:macro pull <instance>
:macro reset <instance>
:macro break <instance>
:macro flatten <instance>
:macro rename <macro> <name>
:macro delete <macro>
:yaml
:apply
:new
:open [path]
:write [path]
:writeas <path>
:undo
:redo
:zoom <percent>
:pan <x> <y>
:help [command]
```

Rack-unique module names, UUIDs prefixed with `@`, and quoted names are valid object references. Ports are addressed by stable manifest IDs. Completion inserts quotes when a name requires them.


Command errors never partially mutate state. Every successful mutating command is one undoable operation unless it explicitly starts an interactive mode.

---

## 13. Visual and interaction design

The implementation must follow the design process in:

<https://github.com/anthropics/claude-code/blob/main/plugins/frontend-design/skills/frontend-design/SKILL.md>

Requirements:

- Modules are visually distinct by automatic category.
- Category color is an accent, not the only category cue.
- Modules use specialized module-owned center controls and displays whenever they clarify state or sound.
- Host chrome remains consistent across built-ins and extensions.
- Signal types remain distinguishable without color.
- All text meets WCAG AA contrast.
- Focus state is always visible.
- Every icon-only button has an accessible name and tooltip.
- Modal focus is trapped and restored on close.
- Reduced-motion mode disables nonessential animation. Clock and live-signal information must remain understandable through static state.
- Telemetry does not cause the entire Rack or entire module panel to rerender. Jacks and custom displays subscribe to the narrowest state slice.
- A module custom-UI error is contained to that module.
- Global and IPC errors appear in a dismissible, deduplicated error center and remain available in a session log.

---

## 14. Initial module set

The first complete release must include:

1. **Oscillator**
   - Note input.
   - Gate input when the selected flavor requires it.
   - Audio output.
   - Sine, saw, triangle, and square shapes.
   - Specialized waveform display.
2. **Volume**
   - Polyphonic Audio input and output.
   - Control level input.
   - Specialized live level display.
   - Bypass route.
3. **ADSR**
   - Gate input.
   - Control envelope output.
   - Interactive envelope in the module-owned center.
4. **Clock**
   - Clock output globally phase-aligned with all Clock modules.
   - Start/stop and frequency controls.
5. **Audio Output**
   - Polyphonic Audio input.
   - Device and physical channel selection.
   - Explicit Speakers and Headphones routing labels.
6. **QWERTY Input**
   - Note and Gate outputs.
   - Receives normalized events in global Keyboard (`k`) mode; every present instance receives the broadcast.
   - Configurable computer-key mapping stored in Rack YAML.
7. **Oscilloscope**
   - Audio input.
   - Trigger and time-window state.
   - Bounded raw capture requested through the host.
8. **EQ**
   - Polyphonic Audio input and output.
   - Interactive response display.
   - Bypass route.
9. **8-channel Mixer**
   - Eight polyphonic Audio inputs.
   - One polyphonic Audio output.
   - Per-input gain, mute, and solo state.
10. **Noise Generator**
    - Audio output.
    - White-noise flavor at minimum.
    - Deterministic saved seed.
11. **Audio File Generator**
    - Gate input.
    - Audio output.
    - Start, retrigger, one-shot, and loop behavior.
    - File path plus content hash in Rack state.
    - Missing-file repair flow.
    - Built-in public-domain examples with attribution included in the app.

Every initial module requires complete documentation, at least one serialized Rack fixture, deterministic offline-render coverage, and custom UI tests where applicable.

---

## 15. Performance targets

Measured on an Apple Silicon Mac representative of the supported baseline:

- End-to-end added engine latency at 48 kHz / 128 frames: at most 10 ms, excluding explicitly declared module lookahead/latency and physical device latency.
- 50 extension modules plus initial built-ins: zero audio-thread allocations and zero xruns in a 10-minute stress run.
- Graph edit publication: no audio interruption longer than one block.
- DSP hot reload: no xrun; old module remains active on failure.
- Telemetry publication: 30 Hz maximum per subscribed display, batched across IPC.
- Rack interaction: 60 fps target while panning and dragging a 100-module Rack at 1440p.
- An idle Rack causes no React rerenders from unchanged telemetry.
- Opening a 100-module Rack from warm caches: under 2 seconds to editable canvas; audio may become ready progressively with status.
- YAML validation and apply for a 100-module Rack: under 200 ms excluding extension compilation.

Wall-clock thresholds in CI must include calibrated headroom. Prefer deterministic counts and bounded-work assertions over fragile stopwatch tests.

---

## 16. Error handling and resilience

- Rust/Tauri command errors are typed and include operation, category, user-safe message, and optional technical detail.
- A rejected command resolves without corrupting frontend state.
- Missing modules render placeholders preserving UUID, YAML state, ports, positions, and unresolved wires.
- Invalid custom state prevents only that module from instantiating.
- WASM faults silence or disable only that instance.
- Native module process crashes cannot be safely contained; the trust warning must state this. On next launch, crash recovery offers to open with third-party native extensions disabled.
- Audio device failure does not close the Rack.
- YAML syntax/schema/semantic errors never replace the last valid graph.
- Repeated identical errors are deduplicated with a count.
- Global window errors and unhandled promise rejections enter the session log.
- Each module panel and custom UI has an error boundary with Retry and Remove/Disable actions.
- Autosave and recovery failures are prominent because they threaten user work.

---

## 17. Security

- Tauri capabilities use least privilege.
- The frontend has no unrestricted shell execution.
- Rack YAML is data, never code.
- YAML parsing disables custom tags and entity-like expansion; anchors/aliases are rejected.
- Extension install validates archive paths, sizes, manifest schema, hashes, and ABI before activation.
- WASM has no ambient capabilities and is resource-bounded.
- Extension React UIs run in the sandbox defined in §11.7.
- Native extensions require explicit trust confirmation and are clearly marked in the picker and module documentation.
- File access for Audio File modules goes through Tauri commands and explicit user selection; extension UIs receive no path capability.

---

## 18. Test strategy and acceptance gates

### 18.0 Universal test and traceability contract

Testing is a product deliverable, not cleanup. This section applies to every normative requirement in `Phase1.md`, `Phase2.md`, `Phase3.md`, and `Phase4.md`.

- Every normative behavior must have an automated test whenever software can observe the outcome. This includes happy paths, boundaries, invalid operations, cancellation, undo/redo, persistence/reopen, migration, keyboard parity, pointer parity, accessibility state, failures, and recovery.
- Every PRD requirement is represented in a checked-in traceability manifest keyed by document heading and stable requirement ID. Each entry names one or more test files/test names or gives a specific human-only justification.
- CI validates the manifest: no requirement may be missing, no named test may be absent, and no automated-testable requirement may be waived as human-only.
- Human-only acceptance is limited to genuinely subjective perception or physical-device feel. It requires a written protocol, expected result, supported platform/device matrix, and recorded release sign-off.
- New behavior is not done until its tests land in the same change. Every fixed defect adds a regression test that fails before the fix whenever reproduction is deterministic.
- Tests exercise real production code paths. Mocks are prohibited when a deterministic fake adapter, fake device, fixture process, null audio device, temporary filesystem, or headless renderer can exercise the boundary honestly.
- Network/provider tests use local deterministic fixture servers or fake provider executables; CI never depends on live YouTube, package services, clocks, or physical MIDI/audio/camera hardware.
- Tests own isolated temporary data roots and cannot read or modify a developer’s Library, Racks, macros, downloads, devices, or global configuration.
- Random/property tests print and persist their seed on failure. Time-dependent tests use virtual time or explicit engine frames wherever possible rather than sleeps.
- No unexplained skip, ignored test, retry-until-green policy, snapshot mass-update, or quarantined failure satisfies a release gate. A platform-gated test names the unavailable capability and still runs its portable core path.
- Prefer semantic assertions and deterministic counts over fragile screenshots and wall-clock thresholds. Use visual snapshots only for geometry/rendering contracts and pair them with DOM/accessibility/route assertions.
- Keep test binaries consolidated by subsystem so expensive engine dependencies link once. During implementation run narrowly scoped cases; CI runs the complete release matrix. Test organization must not trade away coverage for iteration speed.
- Every persistent schema has valid, invalid, oldest-supported, previous-version, and round-trip fixtures. Migration tests operate on copies and prove source data remains untouched until explicit Save.
- Every destructive operation has tests for confirmation, cancellation, partial failure, interruption, rollback, and repeated/idempotent invocation where applicable.
- Every asynchronous command/status flow has ordering, stale-response, rapid-input, cancellation, and shutdown tests.

The implementation plan must derive test work from the requirement manifest before feature work begins. Phase completion requires 100% traceability—not an arbitrary line-coverage percentage—plus the release gates below.

#### Proven suite patterns to carry forward

The greenfield repository should preserve these successful patterns from the current dj-station suite:

- Keep fast crate/library unit tests close to pure code, but consolidate expensive engine integration cases beneath one explicit integration target so heavy audio/runtime dependencies link once.
- Keep serialized full-Rack/golden-audio cases beneath one explicit end-to-end target. A new case is a suite module/fixture, not a new heavyweight test binary.
- Keep real-time safety, calibrated performance, and hot-reload/process-spawning tests as isolated targets because they require a quiet process or special orchestration.
- Organize integration suites by behavior—graph edits, persistence, undo, wiring/merging, QWERTY, macros, playback, analysis, Library, Clip, Grid, modules, telemetry—not by private source file.
- Maintain broad React component/behavior suites for shell shortcuts, wiring, panel layout, collisions, selection, module picker, custom UIs, errors, and each later-phase surface.
- Run heavy Rack/Grid/Clip rendering performance suites after ordinary frontend behavior tests rather than concurrently. Performance assertions prefer touched-item/render counts and asymptotic flat/linear bounds; unavoidable clocks use calibrated generous headroom.
- Every frontend API fixture implements the complete production interface so tests do not accidentally bypass a new behavior. Shared fixture builders should fail compilation/typecheck when an interface grows.
- Run one release/build profile consistently and support one-test-name, one-engine-suite, and one-frontend-file commands for fast iteration. CI alone runs the complete matrix on every change.
- Keep short local and long CI variants of stress tests through an explicit duration variable; the CI duration is a release gate and cannot silently fall back to the local value.

“Copious tests” means broad observable-behavior and edge-case coverage with an efficient suite topology—not duplicated assertions, one binary per scenario, or tests coupled to implementation details.

### 18.1 Unit and property tests

Rust tests must cover:

- Topological sorting and deterministic execution order.
- Wire-router legality, deterministic tie-breaking, and Manhattan optimality. Small random layouts compare against an exhaustive-grid shortest-path oracle; every generated route is checked against every module obstacle and waypoint segment.
- Router invalidation after move/resize/add/delete/macro expand-collapse, including adjacent obstacles, narrow corridors, unreachable targets, crossings, and edge-following lanes.
- Cycle rejection, including paths through input-sync groups and recursively expanded macros.
- Macro dependency-cycle rejection with complete paths, deterministic nested instance IDs, inner-wall interface mapping, and expansion-budget enforcement.
- Every signal merge rule, mono broadcast rule, polyphony cap, and overflow order.
- Input-control order and flavor-range intersection.
- Latency compensation.
- Device configuration fallback decisions.
- Manifest and Rack semantic validation.
- Deterministic unique-name generation and command-name resolution.
- Pure migrations for every supported old format/state version.

Use property tests for graph mutation sequences, merge associativity where the saved-order semantics permit it, YAML round trips, and random valid/invalid endpoint graphs.

### 18.2 DSP conformance

The repository must ship an official module SDK and one conformance suite run against:

- A representative `wasm-2` module.
- A representative `native-2` module.
- A built-in adapter.

The same tests verify buffer layout, active channel counts, connected masks, partial blocks, finite output, state JSON save/load, reset, old-state loading, deterministic seeds, and fault behavior.

### 18.3 Golden audio tests

- Test inputs are complete `.bitfiddle.yaml` files.
- The headless renderer produces WAV output.
- Deterministic cases compare exact sample hashes.
- Algorithms that cannot be bit-identical across supported architectures compare bounded waveform, spectral, and RMS metrics.
- Every new module and every new engine feature adds at least one golden case.
- Required baseline cases include:
  - QWERTY → ADSR → Volume with Oscillator → Volume → Audio Output.
  - Multiple Note sources concatenating polyphonically.
  - Multiple Audio sources concatenating polyphonically.
  - Gate OR and Control addition/broadcast.
  - Clock second-source rejection.
  - Input-sync delivery.
  - Flavor clamping.
  - Bypass.
  - Macro create/instantiate/expand/pull/break.
  - Inner-wall input fan-out and output promotion producing the same audio as a manually wired graph.
  - A three-level nested macro producing sample-identical audio to its fully flattened graph.
  - Audio-file one-shot, retrigger, and loop.
- A documented command intentionally regenerates goldens; CI never regenerates them.

### 18.4 Real-time safety and stress

- A debug allocator tripwire fails on audio-thread allocation/deallocation.
- Instrumented lock/syscall guards fail if reached from the audio thread.
- A 10-minute 48 kHz / 128-frame stress Rack runs with 50 extensions and all initial built-ins, reporting zero xruns on the designated performance machine.
- CI runs a scalable offline equivalent plus a short null-device real-time test.
- Hot reload stress repeatedly replaces WASM and native modules while audio runs and asserts state/wiring preservation, bounded swap time, and no xrun.
- Fault injection covers WASM trap, timeout, invalid active-channel count, NaN output, missing device, and full command queues.

### 18.5 Persistence and migration

Tests must prove:

- Every valid Rack fixture passes the installed JSON Schema and semantic validator.
- Save → load → save is byte-identical when no migration occurs except timestamps/revision under controlled test values.
- Every graphical edit updates only the expected YAML fields.
- Applying YAML is atomic and one undo step.
- Invalid YAML keeps the prior graph playing.
- Atomic-save interruption leaves either the old or new valid file, never a partial file.
- Autosave recovery, external modification conflict, and untitled recovery work.
- Missing module/port/file placeholders preserve unresolved state through another save.
- Older fixtures, including macro format 1, migrate deterministically and never overwrite their source without explicit save.
- Recursive private macro definitions remain playable after deleting every global macro file in their dependency tree.
- Rack files contain all persistent Rack state and tests fail if Rack features write local storage.

### 18.6 UI and keyboard automation

React component tests and Tauri-level end-to-end tests must cover:

- Infinite pan/zoom, reset, and saved viewport.
- 64 px snapping and non-overlap behavior.
- Exact one-unit host border ownership at multiple module sizes/zooms/DPRs: corner actions, non-corner port-only tiles, center clipping/hit testing, module-rendered names, and absence of any title bar.
- Picker search, keyboard navigation, preview fallback, center placement, and drag placement.
- Selection, marquee, group drag, copy/paste, delete, and undo/redo.
- Output wiring, multi-source inputs, input sync, repatching, waypoints, and every rejection reason.
- Pointer drag connection in both directions, threshold click behavior, legal-target highlighting, invalid-target reasons, cancellation by every specified mechanism, pointer-capture loss, and exact undo/redo.
- Wire rendering never intersects module rectangles during preview or committed reroutes; automatic routes are shortest, deterministic, prefer open space on ties, preserve valid waypoints, and render crossings accessibly.
- Every disconnected and connected input behavior for all five signal types.
- YAML edit/apply/error/recovery flow.
- Macro create, inner-wall interface wiring, nested expand/navigation, publish, nested/parent pull semantics, reset, one-level break, recursive flatten, rename, delete, cycle/budget rejection, and orphan behavior.
- Complete keyboard-only wall-port creation, fan-out, rename, reorder, removal, and nesting.
- Sandboxed UI capability enforcement and crash fallback.
- Complete keyboard-only construction and save of the baseline audio Rack.
- Readable command forms, completion, quoted names, and UUID references.
- Keyboard mode entry/exit, all-instance QWERTY broadcast, shortcut suppression, nested recipients, repeat/modifier ordering, blur all-notes-off, and Escape-only exit.
- Focus visibility, modal focus trap, accessible names, contrast, and reduced motion.

No test may require a physical audio or MIDI device. Physical-device feel and latency are separate human acceptance checks.

### 18.7 Release gates

A release is blocked unless:

- The Phase 1–4 requirement traceability manifest is complete, every referenced automated test exists and ran, and every human-only item has recorded sign-off.
- No required test is skipped, ignored, quarantined, or made nonblocking.
- Rust format, lint, tests, and security audit pass.
- TypeScript typecheck, lint, component tests, and end-to-end tests pass.
- Rack, macro, and manifest schemas are installed and their fixtures validate.
- WASM/native conformance passes.
- Golden audio tests pass.
- RT tripwire and CI stress pass.
- The macOS `./run.sh` smoke test launches the app in a clean checkout.
- No known data-loss, graph-corruption, or audio-thread safety defect remains open.

---

## 19. Delivery milestones

### M0 — Shell, schemas, and deterministic headless engine

- Greenfield Tauri/React/Rust workspace.
- `./run.sh` on macOS.
- Requirement traceability manifest/schema and CI validator.
- Consolidated unit, integration, end-to-end/golden, RT, hot-reload, frontend, and performance test targets with scoped local commands.
- Rack/manifest/macro schemas and valid/invalid/migration fixture harness.
- Typed DAG and all signal merge/input rules.
- Headless YAML load and offline WAV render.
- RT-safe null-device engine.
- Minimal built-in Oscillator and Audio Output.

**Exit:** a schema-valid Rack renders deterministic audio offline and through a null/live device without RT violations.

### M1 — Rack editor and persistence

- Infinite 64 px canvas, placement, collision, pan/zoom.
- Strict one-unit host border, system corners, port tiles, and module-owned center with no title bar.
- Generated module panels and signal-specific border controls.
- Bidirectional endpoint drag wiring, input sync, deterministic shortest obstacle-avoiding routing, selection, clipboard, undo/redo.
- One-file YAML view/edit/apply.
- Save/Open/New/Save As, dirty prompts, autosave, recovery, migration shell.
- Keyboard modes, global Escape-only `k` performance mode, and readable command language.

**Exit:** an automated keyboard-only test builds, edits in YAML, saves, reopens, and renders a Rack.

### M2 — Extension platform

- Manifest schema and SDK.
- `wasm-2` host and conformance extension.
- `native-2` host and conformance extension.
- Discovery, installation validation, deprecation, presets, flavors, bypass, state schemas.
- DSP and UI hot reload.
- Sandboxed `ui-2` host with fallback.

**Exit:** WASM and native modules pass identical conformance tests and hot-swap without losing state or wiring.

### M3 — Macros and complete keyboard workflow

- Global recursive macro store and format-2 schema.
- Create/inner-wall interface editor/instantiate/nested expand with breadcrumbs.
- Publish revision, nested and parent Pull Latest semantics, Reset, one-level Break, recursive Flatten, Save as New, rename, delete, orphan handling.
- Dependency-cycle validation, deterministic deep expansion, and explicit depth/size budgets.
- Complete readable command language, search-driven navigation, and on-screen shortcut hints.

**Exit:** wall-authored interfaces, three-level nesting, recursive lifecycle, and complete keyboard-only workflows pass end-to-end tests; saved Racks remain self-contained after deleting every global macro file in their dependency trees.

### M4 — Initial module library and polish

- All modules in §14.
- Specialized custom displays.
- Full documentation.
- Error center and crash recovery.
- Accessibility and reduced-motion pass.
- Performance and 10-minute stress gates.
- Public-domain example audio and attribution.

**Exit:** all release gates in §18.7 pass and human macOS audio latency/feel review finds no blocking issue.

---

## 20. Final acceptance scenario

Starting from a clean macOS checkout, an automated agent must be able to:

1. Run `./run.sh` and observe the Rack.
2. Use only the keyboard to add two QWERTY Inputs, ADSR, Oscillator, Volume, Scope, and Audio Output.
3. Verify each module reserves an exact one-unit host border, has four system-control corners and port-only non-corner border tiles, displays its name in its module-owned center, and has no title bar.
4. Drag from output to input and input to output to wire a playable graph; verify previews and committed shortest routes never cross a module, including while a blocking module moves.
5. Press `k`, verify ordered keydown/up events reach both QWERTY modules while normal shortcuts and clicks do not exit, then verify only Escape exits and releases held notes/gates.
6. Use readable commands to modify the playable graph.
7. Add second Note, Gate, Audio, and Control sources and verify the specified merge behavior.
8. Confirm that adding a second Clock source and creating feedback are rejected without mutation.
9. Synchronize two compatible inputs and verify shared controls.
10. Edit the Rack in YAML, apply it atomically, undo it, and redo it.
11. Save one `.bitfiddle.yaml` file, quit, reopen it, and recover identical Rack state and deterministic offline audio.
12. Create a global macro, expose its input/output by wiring internal ports to the inner walls, and verify external wires use the new ports.
13. Nest that macro inside a second macro, nest the second inside a third, publish and selectively pull revisions, reject a recursive dependency, then delete all three global files and verify the saved Rack still works.
14. Break one level, undo, flatten recursively, and verify the flattened graph is sample-identical.
15. Hot-reload a WASM extension while audio runs without losing wiring/state or reporting an xrun.
16. Render the Rack offline and pass the committed golden comparison.
17. Validate that every Phase 1–4 requirement appears in the checked-in traceability manifest and resolves to an automated test or approved human protocol.
18. Run the complete lint, unit, property, integration, golden, migration, UI, end-to-end, RT tripwire, schema, ABI conformance, stress, and performance matrix successfully with no required skip or quarantine.

Completion of this scenario, plus the release gates, defines v2 done.
