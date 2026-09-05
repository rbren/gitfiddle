export type UUID = string
export type SignalType = 'clock' | 'note' | 'audio' | 'control' | 'gate'
export type PortDirection = 'input' | 'output'
export type GridPoint = { x: number; y: number }

export type ClockInput = { signal: 'clock'; manual_hz: number }
export type NoteInput = { signal: 'note'; manual_hz: number; transpose_semitones: number }
export type AudioInput = {
  signal: 'audio'
  gain: number
  default_source: 'silence' | 'white_noise' | 'sine_440' | 'saw_440' | 'triangle_440' | 'square_440'
  seed: number
}
export type ControlInput = { signal: 'control'; baseline: number; window: number }
export type GateInput = { signal: 'gate'; latched: boolean }
export type InputState = ClockInput | NoteInput | AudioInput | ControlInput | GateInput

export interface Endpoint { module: UUID; port: string }
export interface InputUi { label?: string; color?: string }
export interface ModuleInstance {
  id: UUID
  name: string
  type_id: string
  type_version: string
  abi: 'builtin-2' | 'wasm-2' | 'native-2' | 'missing-2'
  state_version: number
  flavor: string
  position: GridPoint
  bypassed: boolean
  input_ui: Record<string, InputUi>
  inputs: Record<string, InputState>
  state: { parameters: Record<string, unknown>; custom: Record<string, unknown> }
}
export interface Wire {
  id: UUID
  signal: SignalType
  source: Endpoint
  target: Endpoint
  order: number
  waypoints: GridPoint[]
}
export interface InputSync {
  id: UUID
  signal: SignalType
  a: Endpoint
  b: Endpoint
  waypoints: GridPoint[]
}
export interface MacroInstance {
  module_id: UUID
  global_id: UUID
  global_name: string
  format_version: 2
  adopted_revision: number
  adopted_definition: Record<string, unknown>
  current_definition: Record<string, unknown> | null
}
export interface RackDocument {
  format: 'bitfiddle-rack'
  format_version: 2
  app_version: string
  rack: { id: UUID; name: string; revision: number; created_at: string; modified_at: string }
  engine: { sample_rate: number; block_size: number; default_device_id: string | null }
  view: { pan: GridPoint; zoom: number; selected: UUID[] }
  modules: ModuleInstance[]
  wires: Wire[]
  input_sync: InputSync[]
  macros: MacroInstance[]
}
export interface PortDescriptor {
  id: string
  name: string
  signal: SignalType
  direction: PortDirection
  order: number
}
export interface ModuleDescriptor {
  typeId: string
  name: string
  description: string
  category: 'Clock' | 'Effect' | 'Generator' | 'Logic' | 'Mixer' | 'Output' | 'Sequencer' | 'Utility'
  width: number
  height: number
  inputs: PortDescriptor[]
  outputs: PortDescriptor[]
  parameters?: Record<string, unknown>
}
export interface Rect { x: number; y: number; width: number; height: number }
export interface PortRef extends Endpoint {
  direction: PortDirection
  signal: SignalType
}
export type KeyboardMode = 'normal' | 'visual' | 'move' | 'connect' | 'adjust' | 'command' | 'text' | 'keyboard'
