import { descriptorFor } from './modules'
import { moduleRect } from './geometry'
import type { GridPoint, ModuleInstance, PortDescriptor, PortRef, Rect, SignalType } from './types'

export const ROUTE_CLEARANCE = 0.25

type Side = 'top' | 'right' | 'bottom' | 'left'
export interface Anchor extends GridPoint { side: Side }

export function sideFor(port: Pick<PortDescriptor, 'signal' | 'direction'>): Side {
  if (port.signal === 'clock' || port.signal === 'control') return port.direction === 'input' ? 'top' : 'bottom'
  return port.direction === 'input' ? 'left' : 'right'
}

export function anchorFor(module: ModuleInstance, portId: string, direction: 'input' | 'output'): Anchor {
  const descriptor = descriptorFor(module.type_id)
  if (!descriptor) return { ...module.position, side: direction === 'input' ? 'left' : 'right' }
  const allPorts = direction === 'input' ? descriptor.inputs : descriptor.outputs
  const port = allPorts.find((candidate) => candidate.id === portId)
  if (!port) throw new Error(`Unknown port: ${portId}`)
  const side = sideFor(port)
  const ports = allPorts.filter((candidate) => sideFor(candidate) === side)
  const index = ports.findIndex((candidate) => candidate.id === portId)
  const total = side === 'top' || side === 'bottom' ? descriptor.width : descriptor.height
  const available = total - 2
  const offset = 1 + Math.min(available - 1, Math.floor(((index + 0.5) * available) / ports.length)) + 0.5
  if (side === 'top') return { x: module.position.x + offset, y: module.position.y, side }
  if (side === 'bottom') return { x: module.position.x + offset, y: module.position.y + descriptor.height, side }
  if (side === 'left') return { x: module.position.x, y: module.position.y + offset, side }
  return { x: module.position.x + descriptor.width, y: module.position.y + offset, side }
}

export function outward(anchor: Anchor): GridPoint {
  if (anchor.side === 'top') return { x: anchor.x, y: anchor.y - ROUTE_CLEARANCE }
  if (anchor.side === 'bottom') return { x: anchor.x, y: anchor.y + ROUTE_CLEARANCE }
  if (anchor.side === 'left') return { x: anchor.x - ROUTE_CLEARANCE, y: anchor.y }
  return { x: anchor.x + ROUTE_CLEARANCE, y: anchor.y }
}

const expanded = (rect: Rect): Rect => ({ x: rect.x - ROUTE_CLEARANCE, y: rect.y - ROUTE_CLEARANCE, width: rect.width + ROUTE_CLEARANCE * 2, height: rect.height + ROUTE_CLEARANCE * 2 })

function segmentHitsRect(a: GridPoint, b: GridPoint, rect: Rect): boolean {
  if (a.x === b.x) return a.x > rect.x && a.x < rect.x + rect.width && Math.max(Math.min(a.y, b.y), rect.y) < Math.min(Math.max(a.y, b.y), rect.y + rect.height)
  if (a.y === b.y) return a.y > rect.y && a.y < rect.y + rect.height && Math.max(Math.min(a.x, b.x), rect.x) < Math.min(Math.max(a.x, b.x), rect.x + rect.width)
  return true
}

function clearPath(points: GridPoint[], obstacles: Rect[]): boolean {
  return points.slice(1).every((point, index) => obstacles.every((obstacle) => !segmentHitsRect(points[index], point, obstacle)))
}

function compact(points: GridPoint[]): GridPoint[] {
  return points.filter((point, index) => {
    if (index === 0 || index === points.length - 1) return true
    const previous = points[index - 1]
    const next = points[index + 1]
    return !((previous.x === point.x && point.x === next.x) || (previous.y === point.y && point.y === next.y))
  })
}

function routeSegment(start: GridPoint, end: GridPoint, obstacles: Rect[]): GridPoint[] {
  const directCandidates = [[start, { x: end.x, y: start.y }, end], [start, { x: start.x, y: end.y }, end]]
  const direct = directCandidates.filter((candidate) => clearPath(candidate, obstacles)).sort((a, b) => JSON.stringify(a).localeCompare(JSON.stringify(b)))[0]
  if (direct) return compact(direct)
  const xLanes = [...new Set(obstacles.flatMap((rect) => [rect.x, rect.x + rect.width]))].sort((a, b) => Math.abs(a - start.x) - Math.abs(b - start.x) || a - b)
  const yLanes = [...new Set(obstacles.flatMap((rect) => [rect.y, rect.y + rect.height]))].sort((a, b) => Math.abs(a - start.y) - Math.abs(b - start.y) || a - b)
  const candidates: GridPoint[][] = []
  for (const x of xLanes) candidates.push([start, { x, y: start.y }, { x, y: end.y }, end])
  for (const y of yLanes) candidates.push([start, { x: start.x, y }, { x: end.x, y }, end])
  return compact(candidates.filter((candidate) => clearPath(candidate, obstacles)).sort((a, b) => pathLength(a) - pathLength(b) || a.length - b.length || JSON.stringify(a).localeCompare(JSON.stringify(b)))[0] ?? [start, { x: end.x, y: start.y }, end])
}

export function routeWire(modules: ModuleInstance[], source: PortRef, target: PortRef, waypoints: GridPoint[] = []): GridPoint[] {
  const sourceModule = modules.find((module) => module.id === source.module)
  const targetModule = modules.find((module) => module.id === target.module)
  if (!sourceModule || !targetModule) return []
  const startAnchor = anchorFor(sourceModule, source.port, source.direction)
  const endAnchor = anchorFor(targetModule, target.port, target.direction)
  const start = outward(startAnchor)
  const end = outward(endAnchor)
  const obstacles = modules.map(moduleRect).map(expanded)
  const required = [start, ...waypoints, end]
  const route = required.slice(1).flatMap((point, index) => routeSegment(required[index], point, obstacles).slice(index === 0 ? 0 : 1))
  return compact([startAnchor, ...route, endAnchor])
}

export function pathLength(points: GridPoint[]): number {
  return points.slice(1).reduce((total, point, index) => total + Math.abs(point.x - points[index].x) + Math.abs(point.y - points[index].y), 0)
}

export function portsCompatible(a: PortRef, b: PortRef): { valid: boolean; reason?: string } {
  if (a.module === b.module && a.port === b.port && a.direction === b.direction) return { valid: false, reason: 'Choose another port.' }
  if (a.direction === b.direction) return { valid: false, reason: 'Connect an output to an input.' }
  if (a.signal !== b.signal) return { valid: false, reason: `${a.signal} cannot connect to ${b.signal}.` }
  return { valid: true }
}

export function signalStroke(signal: SignalType): string {
  if (signal === 'control') return '#d4483f'
  if (signal === 'gate') return '#59616d'
  if (signal === 'note') return '#7757c7'
  return '#111111'
}
