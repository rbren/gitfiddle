import { createModule, descriptorFor } from './modules'
import { collides, nearestFreePosition } from './geometry'
import { portsCompatible, wouldCreateCycle } from './router'
import { parseRackYaml, rackToYaml } from './yaml'
import type { GridPoint, ModuleInstance, PortRef, RackDocument, UUID } from './types'

const clone = <T,>(value: T): T => structuredClone(value)
const now = () => new Date().toISOString()

export function createEmptyRack(): RackDocument {
  const timestamp = now()
  return {
    format: 'bitfiddle-rack', format_version: 2, app_version: '2.0.0',
    rack: { id: crypto.randomUUID(), name: 'Untitled Rack', revision: 0, created_at: timestamp, modified_at: timestamp },
    engine: { sample_rate: 48000, block_size: 128, default_device_id: null },
    view: { pan: { x: 0, y: 0 }, zoom: 1, selected: [] },
    modules: [], wires: [], input_sync: [], macros: [],
  }
}

type Listener = () => void

export class RackStore {
  private document: RackDocument
  private past: RackDocument[] = []
  private future: RackDocument[] = []
  private listeners = new Set<Listener>()

  constructor(initial: RackDocument = createEmptyRack()) { this.document = clone(initial) }
  getSnapshot = () => this.document
  subscribe = (listener: Listener) => { this.listeners.add(listener); return () => this.listeners.delete(listener) }
  private emit() { this.listeners.forEach((listener) => listener()) }
  private commit(next: RackDocument) {
    this.past = [...this.past.slice(-199), clone(this.document)]
    this.future = []
    this.document = next
    this.emit()
  }
  transact(mutator: (draft: RackDocument) => void) {
    const draft = clone(this.document)
    mutator(draft)
    this.commit(draft)
  }
  setViewport(pan: GridPoint, zoom: number) {
    const draft = clone(this.document)
    draft.view.pan = pan
    draft.view.zoom = zoom
    this.document = draft
    this.emit()
  }
  select(ids: UUID[]) { this.transact((draft) => { draft.view.selected = [...new Set(ids)] }) }
  addModule(typeId: string, target: GridPoint): ModuleInstance {
    const position = nearestFreePosition(this.document.modules, typeId, target)
    const module = createModule(typeId, position, this.document.modules)
    this.transact((draft) => { draft.modules.push(module); draft.view.selected = [module.id] })
    return module
  }
  moveModules(ids: UUID[], delta: GridPoint): boolean {
    const moved = this.document.modules.filter((module) => ids.includes(module.id)).map((module) => ({ ...module, position: { x: module.position.x + delta.x, y: module.position.y + delta.y } }))
    const combined = this.document.modules.map((module) => moved.find((candidate) => candidate.id === module.id) ?? module)
    if (moved.some((module) => collides(combined, module, ids))) return false
    this.transact((draft) => { draft.modules = combined })
    return true
  }
  setModulePositions(positions: Record<UUID, GridPoint>): boolean {
    const ids = Object.keys(positions)
    const combined = this.document.modules.map((module) => positions[module.id] ? { ...module, position: positions[module.id] } : module)
    if (combined.filter((module) => ids.includes(module.id)).some((module) => collides(combined, module, ids))) return false
    this.transact((draft) => { draft.modules = combined })
    return true
  }
  deleteModules(ids: UUID[]) {
    const selected = new Set(ids)
    if (!selected.size) return
    this.transact((draft) => {
      draft.modules = draft.modules.filter((module) => !selected.has(module.id))
      draft.wires = draft.wires.filter((wire) => !selected.has(wire.source.module) && !selected.has(wire.target.module))
      draft.input_sync = draft.input_sync.filter((sync) => !selected.has(sync.a.module) && !selected.has(sync.b.module))
      draft.view.selected = draft.view.selected.filter((id) => !selected.has(id))
    })
  }
  deleteSelected() { this.deleteModules(this.document.view.selected) }
  addWire(a: PortRef, b: PortRef): { valid: boolean; reason?: string } {
    const result = portsCompatible(a, b)
    if (!result.valid) return result
    if (a.direction === 'input' && b.direction === 'input') {
      this.transact((draft) => {
        draft.input_sync.push({ id: crypto.randomUUID(), signal: a.signal, a: { module: a.module, port: a.port }, b: { module: b.module, port: b.port }, waypoints: [] })
        const sourceModule = draft.modules.find((module) => module.id === a.module)
        const targetModule = draft.modules.find((module) => module.id === b.module)
        if (sourceModule && targetModule) targetModule.inputs[b.port] = clone(sourceModule.inputs[a.port])
      })
      return { valid: true }
    }
    const source = a.direction === 'output' ? a : b
    const target = a.direction === 'input' ? a : b
    if (source.signal === 'clock' && this.document.wires.some((wire) => wire.target.module === target.module && wire.target.port === target.port)) return { valid: false, reason: 'Clock inputs accept one source.' }
    if (wouldCreateCycle(this.document.wires, source.module, target.module)) return { valid: false, reason: 'Connection would create a feedback cycle.' }
    this.transact((draft) => draft.wires.push({ id: crypto.randomUUID(), signal: source.signal, source: { module: source.module, port: source.port }, target: { module: target.module, port: target.port }, order: draft.wires.length, waypoints: [] }))
    return { valid: true }
  }
  removeWire(id: UUID) { this.transact((draft) => { draft.wires = draft.wires.filter((wire) => wire.id !== id); draft.input_sync = draft.input_sync.filter((sync) => sync.id !== id) }) }
  serialize() { return rackToYaml(this.document) }
  applyYaml(text: string) { this.commit(parseRackYaml(text)) }
  undo() {
    const previous = this.past.at(-1)
    if (!previous) return false
    this.future = [clone(this.document), ...this.future].slice(0, 200)
    this.document = previous
    this.past = this.past.slice(0, -1)
    this.emit()
    return true
  }
  redo() {
    const next = this.future[0]
    if (!next) return false
    this.past = [...this.past.slice(-199), clone(this.document)]
    this.document = next
    this.future = this.future.slice(1)
    this.emit()
    return true
  }
  portRef(moduleId: UUID, portId: string): PortRef | undefined {
    const module = this.document.modules.find((candidate) => candidate.id === moduleId)
    const descriptor = module && descriptorFor(module.type_id)
    const input = descriptor?.inputs.find((port) => port.id === portId)
    if (input) return { module: moduleId, port: portId, direction: 'input', signal: input.signal }
    const output = descriptor?.outputs.find((port) => port.id === portId)
    return output && { module: moduleId, port: portId, direction: 'output', signal: output.signal }
  }
}

export const rackStore = new RackStore()
