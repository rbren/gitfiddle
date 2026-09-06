import { useEffect, useRef, useState } from 'react'
import { CommandLine } from './components/CommandLine'
import { ModulePicker } from './components/ModulePicker'
import { RackCanvas } from './components/RackCanvas'
import { YamlPanel } from './components/YamlPanel'
import { MODE_HINTS, transitionMode } from './keyboard'
import { clampZoom } from './geometry'
import { rackStore, type RackStore } from './store'
import type { KeyboardMode } from './types'
import { useRack } from './useRack'

export function App({ store = rackStore }: { store?: RackStore }) {
  const document = useRack(store)
  const [mode, setMode] = useState<KeyboardMode>('normal')
  const [pickerOpen, setPickerOpen] = useState(false)
  const [yamlOpen, setYamlOpen] = useState(false)
  const [commandOpen, setCommandOpen] = useState(false)
  const [helpOpen, setHelpOpen] = useState(false)
  const sequence = useRef(0)
  const qwertyCount = document.modules.filter((module) => module.type_id === 'app.qwerty').length

  useEffect(() => {
    const releaseQwerty = () => window.dispatchEvent(new CustomEvent('bitfiddle:qwerty-release-all'))
    const onKey = (event: KeyboardEvent) => {
      const target = event.target
      const editingText = target instanceof HTMLElement && target.matches('input, textarea, [contenteditable="true"]')
      if (editingText && mode !== 'keyboard') return
      if (mode === 'keyboard') {
        event.preventDefault(); event.stopPropagation()
        if (event.key === 'Escape') { releaseQwerty(); setMode('normal'); return }
        sequence.current += 1
        window.dispatchEvent(new CustomEvent('bitfiddle:qwerty', { detail: { code: event.code, key: event.key, kind: event.type === 'keydown' ? 'down' : 'up', repeat: event.repeat, shift: event.shiftKey, control: event.ctrlKey, option: event.altKey, command: event.metaKey, sequence: sequence.current } }))
        return
      }
      if (event.type === 'keyup') return
      const command = event.metaKey || event.ctrlKey
      if (command && ['+', '=', '-', '0'].includes(event.key)) {
        event.preventDefault()
        if (event.key === '0') store.setViewport({ x: 0, y: 0 }, 1)
        else store.setViewport(document.view.pan, clampZoom(document.view.zoom + (event.key === '-' ? -0.1 : 0.1)))
        return
      }
      if (event.key === '?') { event.preventDefault(); setHelpOpen(true); return }
      const transition = transitionMode(mode, event.key, { command, shift: event.shiftKey })
      if (transition.mode !== mode) { event.preventDefault(); setMode(transition.mode); setCommandOpen(transition.mode === 'command') }
      if (!transition.action) return
      event.preventDefault()
      if (transition.action === 'picker') setPickerOpen(true)
      if (transition.action === 'delete') store.deleteSelected()
      if (transition.action === 'undo') store.undo()
      if (transition.action === 'redo') store.redo()
      if (transition.action === 'select-all') store.select(document.modules.map((module) => module.id))
      const moves = { 'move-left': { x: -1, y: 0 }, 'move-right': { x: 1, y: 0 }, 'move-up': { x: 0, y: -1 }, 'move-down': { x: 0, y: 1 } } as const
      if (transition.action in moves) {
        const delta = moves[transition.action as keyof typeof moves]; const amount = event.shiftKey ? 4 : 1
        store.moveModules(document.view.selected, { x: delta.x * amount, y: delta.y * amount })
      }
    }
    const onBlur = () => { if (mode === 'keyboard') releaseQwerty() }
    window.addEventListener('keydown', onKey, true); window.addEventListener('keyup', onKey, true); window.addEventListener('blur', onBlur)
    return () => { window.removeEventListener('keydown', onKey, true); window.removeEventListener('keyup', onKey, true); window.removeEventListener('blur', onBlur) }
  }, [document, mode, store])

  const closeCommand = () => { setCommandOpen(false); setMode('normal') }
  return (
    <main className="app-shell">
      <header className="app-header">
        <div className="brand"><div className="brand-mark">bf</div><div><b>bitfiddle</b><span>Rack editor</span></div></div>
        <div className="rack-title"><span>RACK</span><strong>{document.rack.name}</strong><i>{document.modules.length} modules · {document.wires.length} wires</i></div>
        <div className="header-actions"><button onClick={() => store.undo()} aria-label="Undo">↶</button><button onClick={() => store.redo()} aria-label="Redo">↷</button><button className={yamlOpen ? 'active' : ''} onClick={() => setYamlOpen((open) => !open)}>YAML</button><button className="primary" onClick={() => setPickerOpen(true)}>Add module</button></div>
      </header>
      <section className="workspace">
        <RackCanvas store={store} document={document} onOpenPicker={() => setPickerOpen(true)} onOpenYaml={() => setYamlOpen(true)} />
        {yamlOpen && <YamlPanel store={store} onClose={() => { setYamlOpen(false); setMode('normal') }} onTextFocus={(focused) => setMode(focused ? 'text' : 'normal')} />}
      </section>
      <footer className={`mode-bar mode-${mode}`} role="status" aria-live="polite">
        <b>{mode === 'keyboard' ? 'KEYBOARD k' : mode.toUpperCase()}</b>
        <span>{mode === 'keyboard' ? (qwertyCount ? `${qwertyCount} QWERTY Input recipient${qwertyCount === 1 ? '' : 's'} · Escape exits` : 'No QWERTY Input modules · Escape exits') : MODE_HINTS[mode]}</span>
        <button onClick={() => setHelpOpen(true)}>Shortcuts ?</button>
      </footer>
      {pickerOpen && <ModulePicker onClose={() => setPickerOpen(false)} onPick={(typeId) => { store.addModule(typeId, { x: Math.round(-document.view.pan.x / 64 / document.view.zoom), y: Math.round(-document.view.pan.y / 64 / document.view.zoom) }); setPickerOpen(false) }} />}
      {commandOpen && <CommandLine store={store} onClose={closeCommand} onView={(view) => { if (view === 'yaml') setYamlOpen(true); closeCommand() }} />}
      {helpOpen && <div className="modal-backdrop" onPointerDown={(event) => { if (event.target === event.currentTarget) setHelpOpen(false) }}><section className="help-dialog" role="dialog" aria-modal="true" aria-label="Keyboard help"><header><div><span>REFERENCE</span><h2>Keyboard modes</h2></div><button onClick={() => setHelpOpen(false)}>×</button></header>{Object.entries(MODE_HINTS).map(([name, hint]) => <div key={name}><b>{name}</b><span>{hint}</span></div>)}</section></div>}
    </main>
  )
}
