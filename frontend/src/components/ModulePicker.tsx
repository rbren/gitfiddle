import { useEffect, useMemo, useRef, useState } from 'react'
import { MODULE_DESCRIPTORS } from '../modules'

interface Props { onPick: (typeId: string) => void; onClose: () => void }

export function ModulePicker({ onPick, onClose }: Props) {
  const [query, setQuery] = useState('')
  const [category, setCategory] = useState('All')
  const [highlighted, setHighlighted] = useState(0)
  const search = useRef<HTMLInputElement>(null)
  useEffect(() => search.current?.focus(), [])
  const results = useMemo(() => MODULE_DESCRIPTORS.filter((descriptor) => {
    const matchCategory = category === 'All' || descriptor.category === category
    const haystack = `${descriptor.name} ${descriptor.typeId} ${descriptor.category} ${descriptor.description}`.toLowerCase()
    return matchCategory && haystack.includes(query.toLowerCase())
  }), [category, query])
  useEffect(() => setHighlighted(0), [category, query])
  return (
    <div className="modal-backdrop" onPointerDown={(event) => { if (event.target === event.currentTarget) onClose() }}>
      <section className="module-picker" role="dialog" aria-modal="true" aria-label="Add module">
        <header><div><span>Library</span><h2>Add a module</h2></div><button onClick={onClose} aria-label="Close module picker">×</button></header>
        <input ref={search} aria-label="Search modules" placeholder="Search name, type, category, or description" value={query} onChange={(event) => setQuery(event.target.value)} onKeyDown={(event) => {
          if (event.key === 'ArrowDown') { event.preventDefault(); setHighlighted((value) => Math.min(results.length - 1, value + 1)) }
          if (event.key === 'ArrowUp') { event.preventDefault(); setHighlighted((value) => Math.max(0, value - 1)) }
          if (event.key === 'Enter' && results[highlighted]) onPick(results[highlighted].typeId)
          if (event.key === 'Escape') onClose()
        }} />
        <nav aria-label="Module categories">{['All', ...new Set(MODULE_DESCRIPTORS.map((descriptor) => descriptor.category))].map((item) => <button key={item} className={category === item ? 'active' : ''} onClick={() => setCategory(item)}>{item}</button>)}</nav>
        <div className="picker-results">{results.map((descriptor, index) => (
          <button key={descriptor.typeId} className={`picker-card ${index === highlighted ? 'highlighted' : ''}`} onMouseEnter={() => setHighlighted(index)} onClick={() => onPick(descriptor.typeId)}>
            <div className={`panel-preview category-${descriptor.category.toLowerCase()}`}><i /><strong>{descriptor.name}</strong><span>{descriptor.width}×{descriptor.height}</span></div>
            <div><b>{descriptor.name}</b><code>{descriptor.typeId}</code><p>{descriptor.description}</p></div>
            <em>{descriptor.category}</em>
          </button>
        ))}{results.length === 0 && <p className="empty-state">No matching modules.</p>}</div>
      </section>
    </div>
  )
}
