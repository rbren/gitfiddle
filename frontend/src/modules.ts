import type { InputState, ModuleDescriptor, ModuleInstance, PortDescriptor, SignalType } from './types'

const input = (id: string, name: string, signal: SignalType, order = 0): PortDescriptor => ({ id, name, signal, direction: 'input', order })
const output = (id: string, name: string, signal: SignalType, order = 0): PortDescriptor => ({ id, name, signal, direction: 'output', order })

export const MODULE_DESCRIPTORS: ModuleDescriptor[] = [
  { typeId: 'app.oscillator', name: 'Oscillator', description: 'Polyphonic waveform generator.', category: 'Generator', width: 4, height: 4, inputs: [input('note', 'Note', 'note')], outputs: [output('audio_out', 'Audio Out', 'audio')], parameters: { waveform: 'sine' } },
  { typeId: 'app.volume', name: 'Volume', description: 'Polyphonic level with live control.', category: 'Effect', width: 4, height: 4, inputs: [input('audio_in', 'Audio In', 'audio'), input('level', 'Level', 'control')], outputs: [output('audio_out', 'Audio Out', 'audio')] },
  { typeId: 'app.adsr', name: 'ADSR', description: 'Gate-driven envelope generator.', category: 'Logic', width: 4, height: 4, inputs: [input('gate', 'Gate', 'gate')], outputs: [output('envelope', 'Envelope', 'control')] },
  { typeId: 'app.clock', name: 'Clock', description: 'Globally aligned musical clock.', category: 'Clock', width: 4, height: 4, inputs: [input('rate', 'Rate', 'clock')], outputs: [output('clock_out', 'Clock Out', 'clock')] },
  { typeId: 'app.audio_output', name: 'Audio Output', description: 'Routes incoming sound to a selected device.', category: 'Output', width: 4, height: 4, inputs: [input('audio_in', 'Audio In', 'audio')], outputs: [] },
  { typeId: 'app.noise', name: 'Noise Generator', description: 'Deterministic white-noise generator.', category: 'Generator', width: 4, height: 4, inputs: [], outputs: [output('audio_out', 'Audio Out', 'audio')] },
  { typeId: 'app.qwerty', name: 'QWERTY Input', description: 'Computer-keyboard note and gate input.', category: 'Sequencer', width: 8, height: 4, inputs: [], outputs: [output('note_out', 'Note Out', 'note'), output('gate_out', 'Gate Out', 'gate', 1)] },
  { typeId: 'app.mixer8', name: '8-channel Mixer', description: 'Eight-input polyphonic audio mixer.', category: 'Mixer', width: 4, height: 12, inputs: Array.from({ length: 8 }, (_, index) => input(`in_${index}`, `In ${index + 1}`, 'audio', index)), outputs: [output('audio_out', 'Audio Out', 'audio')] },
  { typeId: 'app.scope', name: 'Oscilloscope', description: 'Live waveform display.', category: 'Output', width: 8, height: 4, inputs: [input('audio_in', 'Audio In', 'audio')], outputs: [] },
]

export const descriptorFor = (typeId: string) => MODULE_DESCRIPTORS.find((descriptor) => descriptor.typeId === typeId)

export const defaultInputState = (signal: SignalType): InputState => {
  switch (signal) {
    case 'clock': return { signal, manual_hz: 2 }
    case 'note': return { signal, manual_hz: 440, transpose_semitones: 0 }
    case 'audio': return { signal, gain: 1, default_source: 'silence', seed: 0 }
    case 'control': return { signal, baseline: 0, window: 1 }
    case 'gate': return { signal, latched: false }
  }
}

export function createModule(typeId: string, position: { x: number; y: number }, existing: ModuleInstance[] = []): ModuleInstance {
  const descriptor = descriptorFor(typeId)
  if (!descriptor) throw new Error(`Unknown module type: ${typeId}`)
  let name = descriptor.name
  let suffix = 2
  while (existing.some((module) => module.name === name)) name = `${descriptor.name} ${suffix++}`
  return {
    id: crypto.randomUUID(),
    name,
    type_id: descriptor.typeId,
    type_version: '2.0.0',
    abi: 'builtin-2',
    state_version: 1,
    flavor: 'Vanilla',
    position,
    bypassed: false,
    input_ui: {},
    inputs: Object.fromEntries(descriptor.inputs.map((port) => [port.id, defaultInputState(port.signal)])),
    state: { parameters: { ...(descriptor.parameters ?? {}) }, custom: {} },
  }
}
