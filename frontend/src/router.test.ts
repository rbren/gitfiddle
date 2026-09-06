import { describe, expect, it } from 'vitest'
import { createModule } from './modules'
import { routeWire } from './router'
import type { GridPoint, Rect } from './types'

function segmentEnters(a: GridPoint, b: GridPoint, rect: Rect): boolean {
  if (a.x === b.x) return a.x > rect.x && a.x < rect.x + rect.width && Math.max(Math.min(a.y, b.y), rect.y) < Math.min(Math.max(a.y, b.y), rect.y + rect.height)
  return a.y > rect.y && a.y < rect.y + rect.height && Math.max(Math.min(a.x, b.x), rect.x) < Math.min(Math.max(a.x, b.x), rect.x + rect.width)
}

describe('wire routing', () => {
  it('routes an orthogonal Manhattan path around module rectangles', () => {
    const source = createModule('app.oscillator', { x: 0, y: 0 })
    const blocker = createModule('app.audio_output', { x: 6, y: 0 }, [source])
    const target = createModule('app.audio_output', { x: 12, y: 0 }, [source, blocker])
    const points = routeWire([source, blocker, target], { module: source.id, port: 'audio_out', direction: 'output', signal: 'audio' }, { module: target.id, port: 'audio_in', direction: 'input', signal: 'audio' })

    expect(points.length).toBeGreaterThan(3)
    expect(points.slice(1).every((point, index) => point.x === points[index].x || point.y === points[index].y)).toBe(true)
    expect(points.slice(1).every((point, index) => !segmentEnters(points[index], point, { x: 6, y: 0, width: 4, height: 4 }))).toBe(true)
  })
})
