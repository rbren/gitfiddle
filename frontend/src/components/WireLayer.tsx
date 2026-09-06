import { GRID_UNIT } from '../geometry'
import { descriptorFor } from '../modules'
import { routeWire, signalStroke } from '../router'
import type { GridPoint, InputSync, PortRef, RackDocument, SignalType, Wire } from '../types'

interface Props {
  document: RackDocument
  pending?: { signal: SignalType; points: GridPoint[] }
  onRemove: (id: string) => void
}

const pointsAttribute = (points: GridPoint[]) => points.map((point) => `${point.x * GRID_UNIT},${point.y * GRID_UNIT}`).join(' ')

function wirePath(document: RackDocument, wire: Wire): GridPoint[] {
  const sourceModule = document.modules.find((module) => module.id === wire.source.module)
  const targetModule = document.modules.find((module) => module.id === wire.target.module)
  const sourcePort = sourceModule && descriptorFor(sourceModule.type_id)?.outputs.find((port) => port.id === wire.source.port)
  const targetPort = targetModule && descriptorFor(targetModule.type_id)?.inputs.find((port) => port.id === wire.target.port)
  if (!sourcePort || !targetPort) return []
  return routeWire(document.modules, { ...wire.source, direction: 'output', signal: wire.signal } as PortRef, { ...wire.target, direction: 'input', signal: wire.signal } as PortRef, wire.waypoints)
}

function syncPath(document: RackDocument, sync: InputSync): GridPoint[] {
  return routeWire(document.modules, { ...sync.a, direction: 'input', signal: sync.signal }, { ...sync.b, direction: 'input', signal: sync.signal }, sync.waypoints)
}

function StyledWire({ signal, points, label, onContextMenu }: { signal: SignalType; points: GridPoint[]; label: string; onContextMenu?: () => void }) {
  if (points.length < 2) return null
  const common = { points: pointsAttribute(points), fill: 'none', strokeLinecap: 'round' as const, strokeLinejoin: 'round' as const }
  return (
    <g className={`wire wire-${signal}`} aria-label={label} role="button" tabIndex={0} onContextMenu={(event) => { event.preventDefault(); onContextMenu?.() }}>
      <polyline {...common} className="wire-hit" stroke="transparent" strokeWidth={18} />
      {signal === 'note' && <>
        <polyline {...common} stroke="hsl(275 70% 48%)" strokeWidth={10} />
        <polyline {...common} stroke="hsl(215 75% 50%)" strokeWidth={5} />
        <polyline {...common} stroke="hsl(35 85% 48%)" strokeWidth={2} />
      </>}
      {signal === 'control' && <>
        <polyline {...common} stroke="#c83535" strokeWidth={2} transform="translate(0 -2)" />
        <polyline {...common} stroke="#24854f" strokeWidth={2} transform="translate(0 2)" />
      </>}
      {signal === 'gate' && <>
        <polyline {...common} stroke="#b5bbc1" strokeWidth={7} />
        <polyline {...common} stroke="#20252b" strokeWidth={3} />
      </>}
      {(signal === 'audio' || signal === 'clock') && <polyline {...common} stroke={signalStroke(signal)} strokeWidth={signal === 'audio' ? 7 : 3} strokeDasharray={signal === 'clock' ? '12 12' : undefined} />}
    </g>
  )
}

export function WireLayer({ document, pending, onRemove }: Props) {
  return (
    <svg className="wire-layer" aria-label="Rack wires">
      {document.wires.map((wire) => <StyledWire key={wire.id} signal={wire.signal} points={wirePath(document, wire)} label={`${wire.signal} wire`} onContextMenu={() => onRemove(wire.id)} />)}
      {document.input_sync.map((sync) => <StyledWire key={sync.id} signal={sync.signal} points={syncPath(document, sync)} label={`${sync.signal} input sync`} onContextMenu={() => onRemove(sync.id)} />)}
      {pending && <StyledWire signal={pending.signal} points={pending.points} label={`Pending ${pending.signal} wire`} />}
    </svg>
  )
}
