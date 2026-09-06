import { render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { createModule } from '../modules'
import { ModulePanel } from './ModulePanel'

describe('module panel ownership', () => {
  it('reserves a one-unit host border with four system corners and module center', () => {
    const module = createModule('app.volume', { x: 2, y: 3 })
    const noop = vi.fn()
    const { container } = render(<ModulePanel module={module} selected={false} onSelect={noop} onDelete={noop} onDragStart={noop} onPortStart={noop} onInfo={noop} onContext={noop} />)
    const panel = container.querySelector<HTMLElement>('.module-panel')!
    expect(panel.style.width).toBe('256px')
    expect(panel.style.height).toBe('256px')
    expect(screen.getByLabelText(`Select and move ${module.name}`)).toBeInTheDocument()
    expect(screen.getByLabelText(`Delete ${module.name}`)).toBeInTheDocument()
    expect(screen.getByLabelText(`Documentation for ${module.name}`)).toBeInTheDocument()
    expect(screen.getByLabelText(`Open context menu for ${module.name}`)).toBeInTheDocument()
    expect(container.querySelector('.module-center')).toHaveTextContent(module.name)
    expect(container.querySelector('header')).not.toBeInTheDocument()
  })
})
