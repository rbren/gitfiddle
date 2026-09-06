import type { CSSProperties, PointerEvent as ReactPointerEvent } from 'react'
import { GRID_UNIT } from '../geometry'
import { descriptorFor } from '../modules'
import { anchorFor, sideFor } from '../router'
import type { ModuleInstance, PortDescriptor, PortRef } from '../types'

interface Props {
  module: ModuleInstance
  selected: boolean
  previewPosition?: { x: number; y: number }
  invalidPosition?: boolean
  onSelect: (module: ModuleInstance, additive: boolean) => void
  onDelete: (module: ModuleInstance) => void
  onDragStart: (event: ReactPointerEvent, module: ModuleInstance) => void
  onPortStart: (event: ReactPointerEvent, port: PortRef) => void
  onInfo: (module: ModuleInstance) => void
  onContext: (event: ReactPointerEvent, module: ModuleInstance) => void
}

function stateLabel(module: ModuleInstance, port: PortDescriptor): string {
  if (port.direction === 'output') return port.name
  const state = module.inputs[port.id]
  if (!state) return port.name
  if (state.signal === 'clock') return `${state.manual_hz} Hz`
  if (state.signal === 'note') return `${Math.round(state.manual_hz)} Hz`
  if (state.signal === 'audio') return `${state.gain.toFixed(1)}×`
  if (state.signal === 'control') return `${state.baseline.toFixed(1)} ±${Math.abs(state.window).toFixed(1)}`
  return state.latched ? 'Latched' : 'Gate'
}

function portStyle(module: ModuleInstance, port: PortDescriptor): CSSProperties {
  const anchor = anchorFor(module, port.id, port.direction)
  const descriptor = descriptorFor(module.type_id)!
  if (anchor.side === 'top') return { left: (anchor.x - module.position.x) * GRID_UNIT - GRID_UNIT / 2, top: 0 }
  if (anchor.side === 'bottom') return { left: (anchor.x - module.position.x) * GRID_UNIT - GRID_UNIT / 2, top: (descriptor.height - 1) * GRID_UNIT }
  if (anchor.side === 'left') return { left: 0, top: (anchor.y - module.position.y) * GRID_UNIT - GRID_UNIT / 2 }
  return { left: (descriptor.width - 1) * GRID_UNIT, top: (anchor.y - module.position.y) * GRID_UNIT - GRID_UNIT / 2 }
}

export function ModulePanel({ module, selected, previewPosition, invalidPosition, onSelect, onDelete, onDragStart, onPortStart, onInfo, onContext }: Props) {
  const descriptor = descriptorFor(module.type_id)
  if (!descriptor) return null
  const position = previewPosition ?? module.position
  const ports = [...descriptor.inputs, ...descriptor.outputs]
  return (
    <article
      className={`module-panel category-${descriptor.category.toLowerCase()} ${selected ? 'selected' : ''} ${invalidPosition ? 'invalid-position' : ''}`}
      style={{ left: position.x * GRID_UNIT, top: position.y * GRID_UNIT, width: descriptor.width * GRID_UNIT, height: descriptor.height * GRID_UNIT }}
      data-module-id={module.id}
      aria-label={`${module.name}, ${descriptor.name}`}
      onPointerDown={(event) => {
        if ((event.target as HTMLElement).closest('.module-center')) onSelect(module, event.shiftKey || event.metaKey || event.ctrlKey)
      }}
    >
      <button className="corner-control drag-control" aria-label={`Select and move ${module.name}`} onPointerDown={(event) => onDragStart(event, module)} onClick={(event) => onSelect(module, event.shiftKey || event.metaKey || event.ctrlKey)}>✥</button>
      <button className="corner-control delete-control" aria-label={`Delete ${module.name}`} onClick={() => onDelete(module)}>×</button>
      <button className="corner-control info-control" aria-label={`Documentation for ${module.name}`} onClick={() => onInfo(module)}>i</button>
      <button className="corner-control menu-control" aria-label={`Open context menu for ${module.name}`} onPointerDown={(event) => onContext(event, module)}>•••</button>
      {ports.map((port) => {
        const side = sideFor(port)
        const ref: PortRef = { module: module.id, port: port.id, direction: port.direction, signal: port.signal }
        return (
          <button
            key={`${port.direction}-${port.id}`}
            className={`port-tile side-${side} signal-${port.signal}`}
            style={portStyle(module, port)}
            aria-label={`${module.name} ${port.name}, ${port.signal} ${port.direction}`}
            title={`${port.name} · ${port.signal} ${port.direction}`}
            data-port-module={module.id}
            data-port-id={port.id}
            data-port-direction={port.direction}
            data-port-signal={port.signal}
            onPointerDown={(event) => onPortStart(event, ref)}
          >
            <span className="port-dot" />
            <small>{stateLabel(module, port)}</small>
          </button>
        )
      })}
      <section className="module-center" aria-label={`${module.name} module interface`}>
        <strong>{module.name}</strong>
        <span>{descriptor.name}</span>
        {module.type_id === 'app.oscillator' && <div className="waveform">∿</div>}
        {module.type_id === 'app.scope' && <div className="scope-line">⌁⌁</div>}
        {module.type_id === 'app.qwerty' && <div className="key-row"><kbd>A</kbd><kbd>W</kbd><kbd>S</kbd><kbd>E</kbd><kbd>D</kbd></div>}
        {module.type_id === 'app.volume' && <div className="meter"><i /></div>}
        {module.bypassed && <em>Bypassed</em>}
      </section>
    </article>
  )
}
