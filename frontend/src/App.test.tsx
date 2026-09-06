import { fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { App } from './App'
import { RackStore, createEmptyRack } from './store'

describe('keyboard modes', () => {
  it('moves through Normal, Visual, Move, Command, and Keyboard modes', () => {
    render(<App store={new RackStore(createEmptyRack())} />)
    expect(screen.getByRole('status')).toHaveTextContent('NORMAL')

    fireEvent.keyDown(window, { key: 'v' })
    expect(screen.getByRole('status')).toHaveTextContent('VISUAL')
    fireEvent.keyDown(window, { key: 'Escape' })
    fireEvent.keyDown(window, { key: 'm' })
    expect(screen.getByRole('status')).toHaveTextContent('MOVE')
    fireEvent.keyDown(window, { key: 'Escape' })
    fireEvent.keyDown(window, { key: ':' })
    expect(screen.getByRole('dialog', { name: 'Rack command' })).toBeInTheDocument()
    fireEvent.keyDown(screen.getByLabelText('Command'), { key: 'Escape' })
    fireEvent.keyDown(window, { key: 'k' })
    expect(screen.getByRole('status')).toHaveTextContent('No QWERTY Input modules · Escape exits')
    fireEvent.keyDown(window, { key: 'a' })
    expect(screen.queryByRole('dialog', { name: 'Add module' })).not.toBeInTheDocument()
    fireEvent.keyDown(window, { key: 'Escape' })
    expect(screen.getByRole('status')).toHaveTextContent('NORMAL')
  })

  it('does not activate Keyboard mode while editing text', () => {
    render(<App store={new RackStore(createEmptyRack())} />)
    fireEvent.click(screen.getAllByRole('button', { name: 'YAML' })[0])
    const editor = screen.getByLabelText('Rack YAML')
    fireEvent.focus(editor)
    fireEvent.keyDown(editor, { key: 'k' })
    expect(screen.getByRole('status')).toHaveTextContent('TEXT')
  })
})
