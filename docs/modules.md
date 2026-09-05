# Built-in module documentation

Every module type documents its name, summary, qualitative behavior,
practical uses, ports, and at least one example patching recipe (PRD §7.4).
This documentation is surfaced from each module's info corner and the module
picker.

All signals are digital: Clock (phase ticks), Note (frequency in Hz), Audio
(normalized amplitude), Control (normalized number), Gate (on/off).

---

## Oscillator (`app.oscillator`)

**Summary:** Polyphonic waveform oscillator.

**Sound:** Pure and steady. Sine is a clean single tone; saw is bright and
buzzy with every harmonic; triangle is mellow with only odd harmonics falling
fast; square is hollow and woody.

**Practical uses:** The primary pitched sound source. Feed it Notes from a
QWERTY Input or sequencer, then shape the result with Volume/ADSR or EQ.

**Ports**

| Port | Direction | Signal | Description |
|---|---|---|---|
| `note` | input (left) | Note | Pitch of each voice. One voice per active Note channel. Disconnected: plays the manual note. |
| `audio_out` | output (right) | Audio | One mono voice per active Note channel. |

**Parameters:** `waveform` — `sine` (default), `saw`, `triangle`, `square`.

**Recipe:** QWERTY Input `note_out` → Oscillator `note`; Oscillator
`audio_out` → Volume `audio_in`; Volume `audio_out` → Audio Output.

---

## Volume (`app.volume`)

**Summary:** Polyphonic amplitude control with a Control-rate level input.

**Behavior:** Multiplies every incoming voice by the delivered level (negative
levels are treated as silence). With ADSR on the level input this is the
classic amplifier envelope.

**Practical uses:** Final loudness control, VCA-style envelope shaping,
ducking, fades.

**Ports**

| Port | Direction | Signal | Description |
|---|---|---|---|
| `audio_in` | input (left) | Audio | Voices to scale. |
| `level` | input (top) | Control | Gain; disconnected delivers baseline. |
| `audio_out` | output (right) | Audio | Scaled voices, same voice count. |

**Bypass:** `audio_out` ← `audio_in`.

**Recipe:** ADSR `envelope` → Volume `level` for an envelope-shaped voice.

---

## ADSR (`app.adsr`)

**Summary:** Attack / decay / sustain / release envelope generator.

**Behavior:** While the gate is on, the envelope ramps to 1 over the attack
time, decays to the sustain level, and holds. When the gate turns off it
releases to 0. One envelope channel per active Gate channel.

**Practical uses:** Shaping amplitude via a Volume level input, controlling
filter or effect depth over time.

**Ports**

| Port | Direction | Signal | Description |
|---|---|---|---|
| `gate` | input (left) | Gate | Trigger/hold. Latched manual gate sustains indefinitely. |
| `envelope` | output (bottom) | Control | 0–1 envelope per channel. |

**Parameters:** `attack`, `decay`, `release` in seconds; `sustain` 0–1.

**Recipe:** QWERTY `gate_out` → ADSR `gate`; ADSR `envelope` → Volume `level`.

---

## Clock (`app.clock`)

**Summary:** Clock source globally phase-aligned with every Clock module.

**Behavior:** Emits phase in `[0, 2π)` derived from the shared process clock
origin. A wrap from high to low phase is a tick. Frequency range 0–40 Hz.

**Practical uses:** Driving sequencers and rhythmic modulation in lockstep;
two Clocks at the same frequency tick together by construction.

**Ports**

| Port | Direction | Signal | Description |
|---|---|---|---|
| `rate` | input (top) | Clock | Disconnected: manual frequency. Connected: follows the incoming clock exactly. |
| `clock_out` | output (bottom) | Clock | Phase output. |

**Recipe:** Clock `clock_out` → sequencer clock inputs; set the manual
frequency on the disconnected `rate` input.

---

## Audio Output (`app.audio_output`)

**Summary:** Physical device output; the only place audio is clipped.

**Behavior:** Concatenated incoming voices are mixed to the selected device
channels. Mono voices copy their left lane to right. Output samples are
clamped to `[-1, 1]` at this boundary only.

**Practical uses:** Every audible Rack terminates here. Multiple Audio Output
modules may target different devices (Speakers, Headphones).

**Ports**

| Port | Direction | Signal | Description |
|---|---|---|---|
| `audio_in` | input (left) | Audio | Voices to play. |

**Recipe:** Volume `audio_out` → Audio Output `audio_in`.

---

## Noise Generator (`app.noise`)

**Summary:** Deterministic white-noise source.

**Sound:** Broadband hiss, equal energy per frequency.

**Practical uses:** Percussion synthesis, texture beds, testing signal paths.

**Ports**

| Port | Direction | Signal | Description |
|---|---|---|---|
| `audio_out` | output (right) | Audio | One mono noise voice. |

**Parameters:** `seed` — deterministic random seed saved in the Rack, so
offline renders are reproducible.

**Recipe:** Noise `audio_out` → Volume `audio_in` with ADSR on `level` for a
snare-style burst.

---

## QWERTY Input (`app.qwerty`)

**Summary:** Computer-keyboard note entry.

**Behavior:** In global Keyboard (`k`) mode every present QWERTY Input
receives every application key event (except Escape) as a broadcast. Held
keys produce Note channels and matching Gate channels. The key mapping is
module state saved in the Rack.

**Practical uses:** Playing patches without MIDI hardware; testing polyphony.

**Ports**

| Port | Direction | Signal | Description |
|---|---|---|---|
| `note_out` | output (right) | Note | One channel per held key. |
| `gate_out` | output (right) | Gate | On while the matching key is held. |

**Recipe:** QWERTY `note_out` → Oscillator `note`; QWERTY `gate_out` → ADSR
`gate`; ADSR `envelope` → Volume `level`.

---

## 8-channel Mixer (`app.mixer8`)

**Summary:** Eight polyphonic Audio inputs with per-input gain, mute, and
solo.

**Behavior:** Voices from unmuted (and, when any solo is active, soloed)
inputs are concatenated in input order after per-input gain, capped at 16
voices.

**Practical uses:** Combining several voices or stems before the output
chain.

**Ports**

| Port | Direction | Signal | Description |
|---|---|---|---|
| `in_0` … `in_7` | inputs (left) | Audio | Sources to combine. |
| `audio_out` | output (right) | Audio | Concatenated voices. |

**Parameters:** `gain_0..7` (linear), `mute_0..7`, `solo_0..7`.

**Recipe:** Two Oscillators and a Noise burst into `in_0..in_2`, mixer out to
Audio Output.

---

## Oscilloscope (`app.scope`)

**Summary:** Waveform display with trigger and time-window state.

**Behavior:** Display-only sink. The UI requests bounded raw capture through
the host; capture never blocks the audio thread.

**Practical uses:** Verifying waveforms, envelopes, and phase relationships.

**Ports**

| Port | Direction | Signal | Description |
|---|---|---|---|
| `audio_in` | input (left) | Audio | Signal to display. |

**Recipe:** Tap any Audio wire by fanning the source out to the scope.
