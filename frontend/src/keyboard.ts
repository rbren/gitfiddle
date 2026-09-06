import type { KeyboardMode } from './types'

export interface ModeTransition {
  mode: KeyboardMode
  action?: 'delete' | 'undo' | 'redo' | 'picker' | 'select-all' | 'move-left' | 'move-right' | 'move-up' | 'move-down'
}

export function transitionMode(mode: KeyboardMode, key: string, modifiers: { command?: boolean; shift?: boolean } = {}): ModeTransition {
  if (mode === 'text') return { mode }
  if (mode === 'keyboard') return key === 'Escape' ? { mode: 'normal' } : { mode }
  if (key === 'Escape') return { mode: 'normal' }
  if (modifiers.command && key.toLowerCase() === 'm') return { mode, action: 'picker' }
  if (modifiers.command && key.toLowerCase() === 'a') return { mode, action: 'select-all' }
  if (modifiers.command && key.toLowerCase() === 'z') return { mode, action: modifiers.shift ? 'redo' : 'undo' }
  if (modifiers.command && key.toLowerCase() === 'y') return { mode, action: 'redo' }
  if (mode === 'move') {
    const directions: Record<string, ModeTransition['action']> = { ArrowLeft: 'move-left', ArrowRight: 'move-right', ArrowUp: 'move-up', ArrowDown: 'move-down' }
    if (directions[key]) return { mode, action: directions[key] }
    if (key === 'Enter') return { mode: 'normal' }
  }
  if (key === 'k') return { mode: 'keyboard' }
  if (key === 'v') return { mode: 'visual' }
  if (key === 'm') return { mode: 'move' }
  if (key === 'c') return { mode: 'connect' }
  if (key === 'Enter' && mode === 'connect') return { mode: 'normal' }
  if (key === 'a' || key === '/') return { mode, action: 'picker' }
  if (key === 'd' || key === 'Backspace' || key === 'Delete') return { mode, action: 'delete' }
  if (key === 'u') return { mode, action: 'undo' }
  if (key === ':') return { mode: 'command' }
  return { mode }
}

export const MODE_HINTS: Record<KeyboardMode, string> = {
  normal: 'Arrows navigate · a add · m move · c connect · v visual · : command · k keyboard',
  visual: 'Space toggles selection · Escape returns',
  move: 'Arrows move by one unit · Shift moves four · Enter commits · Escape cancels',
  connect: 'Choose a matching endpoint · Enter connects · Escape cancels',
  adjust: 'Arrows adjust · 0 resets · Enter commits · Escape cancels',
  command: 'Enter runs the readable command · Escape cancels',
  text: 'Editing text · Escape returns',
  keyboard: 'All keys reach every QWERTY Input · Escape exits',
}
