import { clampZoom } from './geometry'
import type { RackStore } from './store'
import type { ModuleInstance } from './types'

export const COMMAND_GRAMMAR = [
  ':add <type-or-macro> [as <name>] [at <x>,<y>]', ':wire <module>.<output> -> <module>.<input>', ':sync <module>.<input> <-> <module>.<input>',
  ':unwire <wire-or-endpoint>', ':select <module>...', ':set <module>.<input> <signal-specific-value>', ':move <module-or-selection> <x> <y>',
  ':rename <module> <name>', ':flavor <module> <flavor>', ':bypass <module> on|off', ':delete <module-or-selection>', ':copy <module-or-selection>',
  ':paste [at <x>,<y>]', ':macro create <name>', ':macro expose input <module>.<port> as <name>', ':macro expose output <module>.<port> as <name>',
  ':macro unexpose <port>', ':macro publish <instance>', ':macro pull <instance>', ':macro reset <instance>', ':macro break <instance>', ':macro flatten <instance>',
  ':macro rename <macro> <name>', ':macro delete <macro>', ':yaml', ':apply', ':new', ':open [path]', ':write [path]', ':writeas <path>',
  ':undo', ':redo', ':zoom <percent>', ':pan <x> <y>', ':help [command]',
]

function tokens(command: string): string[] {
  return [...command.matchAll(/"([^"]*)"|'([^']*)'|([^\s]+)/g)].map((match) => match[1] ?? match[2] ?? match[3])
}

function findModule(modules: ModuleInstance[], reference: string): ModuleInstance {
  const module = reference.startsWith('@') ? modules.find((candidate) => candidate.id === reference.slice(1)) : modules.find((candidate) => candidate.name === reference)
  if (!module) throw new Error(`Module not found: ${reference}`)
  return module
}

export interface CommandResult { message: string; view?: 'rack' | 'yaml' }

export function executeCommand(store: RackStore, raw: string): CommandResult {
  const parts = tokens(raw.trim().replace(/^:/, ''))
  const [command, ...args] = parts
  const document = store.getSnapshot()
  if (!command) throw new Error('Enter a command.')
  if (command === 'help') return { message: args[0] ? COMMAND_GRAMMAR.find((entry) => entry.startsWith(`:${args[0]}`)) ?? `No help for ${args[0]}.` : COMMAND_GRAMMAR.join('\n') }
  if (command === 'yaml') return { message: 'Opened YAML editor.', view: 'yaml' }
  if (command === 'undo') return { message: store.undo() ? 'Undid the last edit.' : 'Nothing to undo.' }
  if (command === 'redo') return { message: store.redo() ? 'Redid the last edit.' : 'Nothing to redo.' }
  if (command === 'zoom') {
    const percent = Number(args[0])
    if (!Number.isFinite(percent)) throw new Error('Usage: :zoom <percent>')
    store.setViewport(document.view.pan, clampZoom(percent / 100))
    return { message: `Zoomed to ${Math.round(clampZoom(percent / 100) * 100)}%.` }
  }
  if (command === 'pan') {
    const x = Number(args[0]); const y = Number(args[1])
    if (!Number.isFinite(x) || !Number.isFinite(y)) throw new Error('Usage: :pan <x> <y>')
    store.setViewport({ x, y }, document.view.zoom)
    return { message: `Panned to ${x}, ${y}.` }
  }
  if (command === 'add') {
    const typeId = args[0]
    if (!typeId) throw new Error('Usage: :add <type> [as <name>] [at <x>,<y>]')
    const at = args.indexOf('at'); const pair = at >= 0 ? args[at + 1]?.split(',').map(Number) : [0, 0]
    const module = store.addModule(typeId, { x: pair?.[0] ?? 0, y: pair?.[1] ?? 0 })
    const as = args.indexOf('as')
    if (as >= 0 && args[as + 1]) store.transact((draft) => { const added = draft.modules.find((candidate) => candidate.id === module.id); if (added) added.name = args[as + 1] })
    return { message: `Added ${module.name}.` }
  }
  if (command === 'select') {
    store.select(args.map((reference) => findModule(document.modules, reference).id))
    return { message: `Selected ${args.length} module${args.length === 1 ? '' : 's'}.` }
  }
  if (command === 'delete') {
    if (args[0] && args[0] !== 'selection') store.select([findModule(document.modules, args[0]).id])
    store.deleteSelected()
    return { message: 'Deleted selection.' }
  }
  if (command === 'rename') {
    const module = findModule(document.modules, args[0]); const name = args.slice(1).join(' ')
    if (!name) throw new Error('Usage: :rename <module> <name>')
    if (document.modules.some((candidate) => candidate.id !== module.id && candidate.name === name)) throw new Error(`Name already exists: ${name}`)
    store.transact((draft) => { draft.modules.find((candidate) => candidate.id === module.id)!.name = name })
    return { message: `Renamed ${module.name} to ${name}.` }
  }
  if (command === 'bypass') {
    const module = findModule(document.modules, args[0]); const enabled = args[1] === 'on'
    if (!['on', 'off'].includes(args[1])) throw new Error('Usage: :bypass <module> on|off')
    store.transact((draft) => { draft.modules.find((candidate) => candidate.id === module.id)!.bypassed = enabled })
    return { message: `${module.name} bypass ${enabled ? 'on' : 'off'}.` }
  }
  if (command === 'move') {
    const module = args[0] === 'selection' ? null : findModule(document.modules, args[0]); const x = Number(args[1]); const y = Number(args[2])
    if (!Number.isInteger(x) || !Number.isInteger(y)) throw new Error('Usage: :move <module-or-selection> <x> <y>')
    const ids = module ? [module.id] : document.view.selected
    if (!store.moveModules(ids, { x, y })) throw new Error('Move rejected because modules would overlap.')
    return { message: `Moved ${ids.length} module${ids.length === 1 ? '' : 's'}.` }
  }
  if (command === 'wire') {
    const arrow = args.indexOf('->')
    if (arrow !== 1 || !args[2]) throw new Error('Usage: :wire <module>.<output> -> <module>.<input>')
    const parseEndpoint = (value: string) => { const split = value.lastIndexOf('.'); if (split < 1) throw new Error(`Invalid endpoint: ${value}`); return [value.slice(0, split), value.slice(split + 1)] as const }
    const [sourceName, sourcePort] = parseEndpoint(args[0]); const [targetName, targetPort] = parseEndpoint(args[2])
    const source = findModule(document.modules, sourceName); const target = findModule(document.modules, targetName)
    const sourceRef = store.portRef(source.id, sourcePort); const targetRef = store.portRef(target.id, targetPort)
    if (!sourceRef || !targetRef) throw new Error('Unknown wire endpoint.')
    const result = store.addWire(sourceRef, targetRef); if (!result.valid) throw new Error(result.reason)
    return { message: `Wired ${args[0]} to ${args[2]}.` }
  }
  throw new Error(`Unsupported command: ${command}. Use :help for readable forms.`)
}
