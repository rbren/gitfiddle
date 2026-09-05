import { descriptorFor } from './modules'
import type { GridPoint, ModuleInstance, Rect } from './types'

export const GRID_UNIT = 64
export const MIN_ZOOM = 0.05
export const MAX_ZOOM = 2.5

export const snapToGrid = (pixels: number, zoom = 1) => Math.round(pixels / (GRID_UNIT * zoom))
export const clampZoom = (zoom: number) => Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, zoom))

export function moduleRect(module: ModuleInstance): Rect {
  const descriptor = descriptorFor(module.type_id)
  if (!descriptor) return { x: module.position.x, y: module.position.y, width: 4, height: 4 }
  return { x: module.position.x, y: module.position.y, width: descriptor.width, height: descriptor.height }
}

export function rectanglesOverlap(a: Rect, b: Rect): boolean {
  return a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height
}

export function collides(modules: ModuleInstance[], candidate: ModuleInstance, ignoredIds: string[] = []): boolean {
  const ignored = new Set([candidate.id, ...ignoredIds])
  const rect = moduleRect(candidate)
  return modules.some((module) => !ignored.has(module.id) && rectanglesOverlap(rect, moduleRect(module)))
}

export function nearestFreePosition(modules: ModuleInstance[], typeId: string, target: GridPoint): GridPoint {
  const descriptor = descriptorFor(typeId)
  if (!descriptor) return target
  for (let radius = 0; radius < 200; radius += 1) {
    for (let y = -radius; y <= radius; y += 1) {
      for (let x = -radius; x <= radius; x += 1) {
        if (radius > 0 && Math.abs(x) !== radius && Math.abs(y) !== radius) continue
        const rect = { x: target.x + x, y: target.y + y, width: descriptor.width, height: descriptor.height }
        if (!modules.some((module) => rectanglesOverlap(rect, moduleRect(module)))) return { x: rect.x, y: rect.y }
      }
    }
  }
  return target
}
