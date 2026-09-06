import { describe, expect, it } from 'vitest'
import { snapToGrid } from './geometry'
import { portsCompatible } from './router'
import { RackStore, createEmptyRack } from './store'
import { parseRackYaml, rackToYaml } from './yaml'
import type { PortRef } from './types'

describe('rack geometry', () => {
  it('snaps screen movement to 64px grid units', () => {
    expect(snapToGrid(31)).toBe(0)
    expect(snapToGrid(33)).toBe(1)
    expect(snapToGrid(-97)).toBe(-2)
    expect(snapToGrid(64, 0.5)).toBe(2)
  })

  it('rejects a module move that would overlap another module', () => {
    const store = new RackStore(createEmptyRack())
    const oscillator = store.addModule('app.oscillator', { x: 0, y: 0 })
    const output = store.addModule('app.audio_output', { x: 8, y: 0 })

    expect(store.setModulePositions({ [output.id]: { x: 2, y: 0 } })).toBe(false)
    expect(store.getSnapshot().modules.find((module) => module.id === oscillator.id)?.position).toEqual({ x: 0, y: 0 })
    expect(store.getSnapshot().modules.find((module) => module.id === output.id)?.position).toEqual({ x: 8, y: 0 })
  })
})

describe('typed wires', () => {
  const output = (signal: PortRef['signal']): PortRef => ({ module: crypto.randomUUID(), port: 'out', direction: 'output', signal })
  const input = (signal: PortRef['signal']): PortRef => ({ module: crypto.randomUUID(), port: 'in', direction: 'input', signal })

  it('accepts matching output-input and input-sync endpoints', () => {
    expect(portsCompatible(output('audio'), input('audio')).valid).toBe(true)
    expect(portsCompatible(input('note'), input('note')).valid).toBe(true)
    expect(portsCompatible(output('note'), input('gate'))).toMatchObject({ valid: false })
    expect(portsCompatible(output('control'), output('control'))).toMatchObject({ valid: false })
  })

  it('creates input sync and rejects a connection that closes a cycle', () => {
    const store = new RackStore(createEmptyRack())
    const firstOscillator = store.addModule('app.oscillator', { x: 0, y: 0 })
    const secondOscillator = store.addModule('app.oscillator', { x: 8, y: 0 })
    const firstNote = store.portRef(firstOscillator.id, 'note')!
    const secondNote = store.portRef(secondOscillator.id, 'note')!
    expect(store.addWire(firstNote, secondNote).valid).toBe(true)
    expect(store.getSnapshot().input_sync).toHaveLength(1)

    const firstVolume = store.addModule('app.volume', { x: 0, y: 8 })
    const secondVolume = store.addModule('app.volume', { x: 8, y: 8 })
    expect(store.addWire(store.portRef(firstVolume.id, 'audio_out')!, store.portRef(secondVolume.id, 'audio_in')!).valid).toBe(true)
    expect(store.addWire(store.portRef(secondVolume.id, 'audio_out')!, store.portRef(firstVolume.id, 'audio_in')!)).toMatchObject({ valid: false })
    expect(store.getSnapshot().wires).toHaveLength(1)
  })
})

describe('rack YAML', () => {
  it('round-trips the exact rack document shape', () => {
    const store = new RackStore(createEmptyRack())
    store.addModule('app.oscillator', { x: -4, y: 3 })
    store.addModule('app.audio_output', { x: 4, y: 3 })
    const yaml = store.serialize()
    const parsed = parseRackYaml(yaml)

    expect(parsed).toEqual(store.getSnapshot())
    expect(rackToYaml(parsed)).toBe(yaml)
    expect(yaml.endsWith('\n')).toBe(true)
  })

  it('applies valid YAML as one undoable transaction', () => {
    const store = new RackStore(createEmptyRack())
    const yaml = store.serialize().replace('Untitled Rack', 'Applied Rack')
    store.applyYaml(yaml)
    expect(store.getSnapshot().rack.name).toBe('Applied Rack')
    expect(store.undo()).toBe(true)
    expect(store.getSnapshot().rack.name).toBe('Untitled Rack')
  })
})
