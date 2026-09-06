import { useEffect, useRef, useState } from 'react'
import { COMMAND_GRAMMAR, executeCommand } from '../commands'
import type { RackStore } from '../store'

interface Props { store: RackStore; onClose: () => void; onView: (view: 'rack' | 'yaml') => void }

export function CommandLine({ store, onClose, onView }: Props) {
  const [value, setValue] = useState(':')
  const [message, setMessage] = useState('')
  const input = useRef<HTMLInputElement>(null)
  useEffect(() => input.current?.focus(), [])
  const run = () => {
    try { const result = executeCommand(store, value); setMessage(result.message); if (result.view) onView(result.view); if (!result.view) onClose() }
    catch (reason) { setMessage(reason instanceof Error ? reason.message : String(reason)) }
  }
  return (
    <div className="command-line" role="dialog" aria-label="Rack command">
      <span>COMMAND</span>
      <input ref={input} aria-label="Command" value={value} list="command-grammar" onChange={(event) => { setValue(event.target.value); setMessage('') }} onKeyDown={(event) => { if (event.key === 'Enter') run(); if (event.key === 'Escape') onClose() }} />
      <datalist id="command-grammar">{COMMAND_GRAMMAR.map((command) => <option key={command} value={command} />)}</datalist>
      {message && <output>{message}</output>}
      <button onClick={run}>Run</button>
    </div>
  )
}
