import { useEffect, useState } from 'react'
import type { KeyboardEvent } from 'react'
import type { RackStore } from '../store'
import { useRack } from '../useRack'

interface Props { store: RackStore; onClose: () => void; onTextFocus: (focused: boolean) => void }

export function YamlPanel({ store, onClose, onTextFocus }: Props) {
  const document = useRack(store)
  const serialized = store.serialize()
  const [draft, setDraft] = useState(serialized)
  const [error, setError] = useState('')
  const [dirty, setDirty] = useState(false)

  useEffect(() => { if (!dirty) setDraft(serialized) }, [document, dirty, serialized])
  const apply = () => {
    try { store.applyYaml(draft); setError(''); setDirty(false) }
    catch (reason) { setError(reason instanceof Error ? reason.message : String(reason)) }
  }
  const onKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if ((event.metaKey || event.ctrlKey) && event.key === 'Enter') { event.preventDefault(); apply() }
  }
  return (
    <aside className="yaml-panel" aria-label="Rack YAML editor">
      <header><div><span>Document</span><h2>Rack YAML</h2></div><button onClick={onClose} aria-label="Close YAML editor">×</button></header>
      <textarea aria-label="Rack YAML" value={draft} spellCheck={false} onFocus={() => onTextFocus(true)} onBlur={() => onTextFocus(false)} onKeyDown={onKeyDown} onChange={(event) => { setDraft(event.target.value); setDirty(true); setError('') }} />
      {error && <p className="yaml-error" role="alert">{error}</p>}
      <footer><span>{dirty ? 'Draft not applied' : 'Graph and YAML synchronized'}</span><button className="primary" onClick={apply}>Apply</button></footer>
    </aside>
  )
}
