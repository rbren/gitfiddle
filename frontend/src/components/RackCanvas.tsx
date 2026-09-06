import { useMemo, useRef, useState } from 'react'
import type { PointerEvent as ReactPointerEvent, WheelEvent as ReactWheelEvent } from 'react'
import { clampZoom, GRID_UNIT, moduleRect, rectanglesOverlap } from '../geometry'
import { descriptorFor } from '../modules'
import { portsCompatible, routeToPoint, routeWire } from '../router'
import type { GridPoint, ModuleInstance, PortRef, RackDocument } from '../types'
import type { RackStore } from '../store'
import { ModulePanel } from './ModulePanel'
import { WireLayer } from './WireLayer'

interface Props { store: RackStore; document: RackDocument; onOpenPicker: () => void; onOpenYaml: () => void }
interface DragPreview { positions: Record<string, GridPoint>; invalid: boolean }
interface Marquee { x: number; y: number; width: number; height: number }
interface MenuState { x: number; y: number; module?: ModuleInstance }

const isAdditive = (event: { shiftKey: boolean; metaKey: boolean; ctrlKey: boolean }) => event.shiftKey || event.metaKey || event.ctrlKey

export function RackCanvas({ store, document, onOpenPicker, onOpenYaml }: Props) {
  const canvas = useRef<HTMLDivElement>(null)
  const [drag, setDrag] = useState<DragPreview | null>(null)
  const [marquee, setMarquee] = useState<Marquee | null>(null)
  const [wiring, setWiring] = useState<{ source: PortRef; points: GridPoint[]; active: boolean } | null>(null)
  const [menu, setMenu] = useState<MenuState | null>(null)
  const [notice, setNotice] = useState('Ready')
  const displayDocument = useMemo(() => drag ? { ...document, modules: document.modules.map((module) => drag.positions[module.id] ? { ...module, position: drag.positions[module.id] } : module) } : document, [document, drag])

  const selectModule = (module: ModuleInstance, additive: boolean) => {
    if (additive) store.select(document.view.selected.includes(module.id) ? document.view.selected.filter((id) => id !== module.id) : [...document.view.selected, module.id])
    else if (!document.view.selected.includes(module.id) || document.view.selected.length !== 1) store.select([module.id])
  }

  const startModuleDrag = (event: ReactPointerEvent, module: ModuleInstance) => {
    if (event.button !== 0) return
    event.stopPropagation()
    const ids = document.view.selected.includes(module.id) ? document.view.selected : [module.id]
    if (!document.view.selected.includes(module.id)) store.select(ids)
    const origins = Object.fromEntries(document.modules.filter((candidate) => ids.includes(candidate.id)).map((candidate) => [candidate.id, candidate.position]))
    const start = { x: event.clientX, y: event.clientY }
    const move = (pointer: PointerEvent) => {
      const delta = { x: Math.round((pointer.clientX - start.x) / (GRID_UNIT * document.view.zoom)), y: Math.round((pointer.clientY - start.y) / (GRID_UNIT * document.view.zoom)) }
      const positions = Object.fromEntries(Object.entries(origins).map(([id, point]) => [id, { x: point.x + delta.x, y: point.y + delta.y }]))
      const moving = document.modules.filter((candidate) => ids.includes(candidate.id)).map((candidate) => ({ ...candidate, position: positions[candidate.id] }))
      const stationary = document.modules.filter((candidate) => !ids.includes(candidate.id))
      const invalid = moving.some((candidate) => stationary.some((other) => rectanglesOverlap(moduleRect(candidate), moduleRect(other))))
      setDrag({ positions, invalid })
    }
    const up = () => {
      setDrag((current) => { if (current && !current.invalid) store.setModulePositions(current.positions); else if (current?.invalid) setNotice('Move rejected: modules cannot overlap.'); return null })
      window.removeEventListener('pointermove', move); window.removeEventListener('pointerup', up); window.removeEventListener('pointercancel', cancel)
    }
    const cancel = () => { setDrag(null); window.removeEventListener('pointermove', move); window.removeEventListener('pointerup', up); window.removeEventListener('pointercancel', cancel) }
    window.addEventListener('pointermove', move); window.addEventListener('pointerup', up); window.addEventListener('pointercancel', cancel)
  }

  const startPort = (event: ReactPointerEvent, source: PortRef) => {
    if (event.button !== 0) return
    event.stopPropagation(); event.preventDefault()
    const start = { x: event.clientX, y: event.clientY }
    setWiring({ source, points: [], active: false })
    const move = (pointer: PointerEvent) => {
      const rect = canvas.current?.getBoundingClientRect(); if (!rect) return
      const target = { x: (pointer.clientX - rect.left - document.view.pan.x) / document.view.zoom / GRID_UNIT, y: (pointer.clientY - rect.top - document.view.pan.y) / document.view.zoom / GRID_UNIT }
      const active = Math.hypot(pointer.clientX - start.x, pointer.clientY - start.y) > 4
      setWiring({ source, active, points: active ? routeToPoint(document.modules, source, target) : [] })
    }
    const finish = (pointer: PointerEvent) => {
      const target = globalThis.document.elementFromPoint(pointer.clientX, pointer.clientY)?.closest<HTMLElement>('[data-port-id]')
      if (target && Math.hypot(pointer.clientX - start.x, pointer.clientY - start.y) > 4) {
        const destination: PortRef = { module: target.dataset.portModule!, port: target.dataset.portId!, direction: target.dataset.portDirection as PortRef['direction'], signal: target.dataset.portSignal as PortRef['signal'] }
        const compatibility = portsCompatible(source, destination)
        if (compatibility.valid) {
          const result = store.addWire(source, destination); setNotice(result.valid ? `Connected ${source.signal} ports.` : result.reason ?? 'Connection rejected.')
        } else setNotice(compatibility.reason ?? 'Connection rejected.')
      }
      setWiring(null); cleanup()
    }
    const cancel = () => { setWiring(null); cleanup() }
    const cleanup = () => { window.removeEventListener('pointermove', move); window.removeEventListener('pointerup', finish); window.removeEventListener('pointercancel', cancel); window.removeEventListener('blur', cancel) }
    window.addEventListener('pointermove', move); window.addEventListener('pointerup', finish); window.addEventListener('pointercancel', cancel); window.addEventListener('blur', cancel)
  }

  const backgroundPointerDown = (event: ReactPointerEvent<HTMLDivElement>) => {
    if ((event.target as HTMLElement).closest('.module-panel, .wire, .canvas-toolbar, .context-menu')) return
    setMenu(null)
    const rect = canvas.current!.getBoundingClientRect()
    const start = { x: event.clientX - rect.left, y: event.clientY - rect.top }
    if (event.button === 1 || event.altKey) {
      event.preventDefault()
      const initialPan = document.view.pan
      const move = (pointer: PointerEvent) => store.setViewport({ x: initialPan.x + pointer.clientX - event.clientX, y: initialPan.y + pointer.clientY - event.clientY }, document.view.zoom)
      const up = () => { window.removeEventListener('pointermove', move); window.removeEventListener('pointerup', up) }
      window.addEventListener('pointermove', move); window.addEventListener('pointerup', up)
      return
    }
    if (event.button !== 0) return
    const move = (pointer: PointerEvent) => {
      const current = { x: pointer.clientX - rect.left, y: pointer.clientY - rect.top }
      setMarquee({ x: Math.min(start.x, current.x), y: Math.min(start.y, current.y), width: Math.abs(current.x - start.x), height: Math.abs(current.y - start.y) })
    }
    const up = (pointer: PointerEvent) => {
      const current = { x: pointer.clientX - rect.left, y: pointer.clientY - rect.top }
      const box = { x: Math.min(start.x, current.x), y: Math.min(start.y, current.y), width: Math.abs(current.x - start.x), height: Math.abs(current.y - start.y) }
      const hits = document.modules.filter((module) => {
        const bounds = moduleRect(module)
        return rectanglesOverlap(box, { x: document.view.pan.x + bounds.x * GRID_UNIT * document.view.zoom, y: document.view.pan.y + bounds.y * GRID_UNIT * document.view.zoom, width: bounds.width * GRID_UNIT * document.view.zoom, height: bounds.height * GRID_UNIT * document.view.zoom })
      }).map((module) => module.id)
      store.select(isAdditive(event) ? [...new Set([...document.view.selected, ...hits])] : hits)
      setMarquee(null); window.removeEventListener('pointermove', move); window.removeEventListener('pointerup', up)
    }
    window.addEventListener('pointermove', move); window.addEventListener('pointerup', up)
  }

  const onWheel = (event: ReactWheelEvent) => {
    event.preventDefault()
    if (event.ctrlKey || event.metaKey) {
      const rect = canvas.current!.getBoundingClientRect(); const nextZoom = clampZoom(document.view.zoom * Math.exp(-event.deltaY * 0.002))
      const point = { x: event.clientX - rect.left, y: event.clientY - rect.top }
      const world = { x: (point.x - document.view.pan.x) / document.view.zoom, y: (point.y - document.view.pan.y) / document.view.zoom }
      store.setViewport({ x: point.x - world.x * nextZoom, y: point.y - world.y * nextZoom }, nextZoom)
    } else store.setViewport({ x: document.view.pan.x - event.deltaX, y: document.view.pan.y - event.deltaY }, document.view.zoom)
  }

  const resetView = () => store.setViewport({ x: 0, y: 0 }, 1)
  const centerTarget = () => {
    const rect = canvas.current?.getBoundingClientRect()
    return { x: Math.round(((rect?.width ?? 800) / 2 - document.view.pan.x) / document.view.zoom / GRID_UNIT), y: Math.round(((rect?.height ?? 600) / 2 - document.view.pan.y) / document.view.zoom / GRID_UNIT) }
  }

  return (
    <div ref={canvas} className={`rack-canvas ${wiring?.active ? `wiring wiring-${wiring.source.signal} wiring-from-${wiring.source.direction}` : ''}`} style={{ '--grid-size': `${GRID_UNIT * document.view.zoom * (document.view.zoom < 0.2 ? 4 : 1)}px`, '--grid-x': `${document.view.pan.x}px`, '--grid-y': `${document.view.pan.y}px` } as React.CSSProperties} onPointerDown={backgroundPointerDown} onWheel={onWheel} onContextMenu={(event) => { if (!(event.target as HTMLElement).closest('.module-panel, .wire')) { event.preventDefault(); setMenu({ x: event.clientX, y: event.clientY }) } }}>
      <div className="canvas-toolbar"><button onClick={onOpenPicker}>+ Module <kbd>⌘M</kbd></button><button onClick={onOpenYaml}>YAML</button><span>{Math.round(document.view.zoom * 100)}%</span><button onClick={() => store.setViewport(document.view.pan, clampZoom(document.view.zoom - 0.1))}>−</button><button onClick={resetView}>Reset</button><button onClick={() => store.setViewport(document.view.pan, clampZoom(document.view.zoom + 0.1))}>+</button></div>
      <div className="rack-world" style={{ transform: `translate(${document.view.pan.x}px, ${document.view.pan.y}px) scale(${document.view.zoom})` }}>
        <WireLayer document={displayDocument} pending={wiring?.active ? { signal: wiring.source.signal, points: wiring.points } : undefined} onRemove={(id) => store.removeWire(id)} />
        {document.modules.map((module) => <ModulePanel key={module.id} module={module} selected={document.view.selected.includes(module.id)} previewPosition={drag?.positions[module.id]} invalidPosition={Boolean(drag?.positions[module.id] && drag.invalid)} onSelect={selectModule} onDelete={(target) => store.deleteModules([target.id])} onDragStart={startModuleDrag} onPortStart={startPort} onInfo={(target) => setNotice(descriptorFor(target.type_id)?.description ?? target.name)} onContext={(event, target) => { event.stopPropagation(); setMenu({ x: event.clientX, y: event.clientY, module: target }) }} />)}
      </div>
      {marquee && <div className="marquee" style={marquee} />}
      {menu && <div className="context-menu" style={{ left: menu.x, top: menu.y }}>{menu.module ? <><b>{menu.module.name}</b><button onClick={() => { store.select([menu.module!.id]); setMenu(null) }}>Select</button><button onClick={() => { setNotice(descriptorFor(menu.module!.type_id)?.description ?? ''); setMenu(null) }}>Documentation</button><button className="danger" onClick={() => { store.deleteModules([menu.module!.id]); setMenu(null) }}>Delete</button></> : <><b>Rack</b><button onClick={() => { onOpenPicker(); setMenu(null) }}>Add Module</button><button onClick={() => { onOpenYaml(); setMenu(null) }}>View/Edit YAML</button><button onClick={() => { const position = centerTarget(); store.addModule('app.oscillator', position); setMenu(null) }}>Add Oscillator</button></>}</div>}
      <div className="canvas-notice" aria-live="polite">{notice}</div>
    </div>
  )
}
