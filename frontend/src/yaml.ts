import Ajv2020 from 'ajv/dist/2020'
import addFormats from 'ajv-formats'
import { dump, load } from 'js-yaml'
import rackSchema from '../../schemas/rack.schema.json'
import { descriptorFor } from './modules'
import { moduleRect, rectanglesOverlap } from './geometry'
import { portsCompatible } from './router'
import type { PortRef, RackDocument } from './types'

const ajv = new Ajv2020({ allErrors: true, strict: false })
addFormats(ajv)
const validateSchema = ajv.compile(rackSchema)

export function rackToYaml(document: RackDocument): string {
  return dump(document, { indent: 2, lineWidth: -1, noRefs: true, noCompatMode: true, sortKeys: false })
}

export function parseRackYaml(text: string): RackDocument {
  if (/(^|:\s*)[&*][A-Za-z0-9_-]+/m.test(text)) throw new Error('YAML anchors and aliases are not allowed.')
  const candidate = load(text, { json: true })
  if (!validateSchema(candidate)) throw new Error(validateSchema.errors?.map((error) => `${error.instancePath || '/'} ${error.message}`).join('; ') ?? 'Rack schema validation failed.')
  const document = candidate as RackDocument
  validateSemantics(document)
  return document
}

export function validateSemantics(document: RackDocument): void {
  const moduleIds = new Set<string>()
  const names = new Set<string>()
  for (const module of document.modules) {
    if (moduleIds.has(module.id)) throw new Error(`Duplicate module id: ${module.id}`)
    if (names.has(module.name)) throw new Error(`Duplicate module name: ${module.name}`)
    if (!descriptorFor(module.type_id) && module.abi !== 'missing-2') throw new Error(`Unknown module type: ${module.type_id}`)
    moduleIds.add(module.id)
    names.add(module.name)
  }
  for (let index = 0; index < document.modules.length; index += 1) {
    for (let other = index + 1; other < document.modules.length; other += 1) {
      if (rectanglesOverlap(moduleRect(document.modules[index]), moduleRect(document.modules[other]))) throw new Error(`Modules overlap: ${document.modules[index].name} and ${document.modules[other].name}`)
    }
  }
  for (const selected of document.view.selected) if (!moduleIds.has(selected)) throw new Error(`Selected module does not exist: ${selected}`)
  const wireIds = new Set<string>()
  for (const wire of document.wires) {
    if (wireIds.has(wire.id)) throw new Error(`Duplicate wire id: ${wire.id}`)
    wireIds.add(wire.id)
    const sourceModule = document.modules.find((module) => module.id === wire.source.module)
    const targetModule = document.modules.find((module) => module.id === wire.target.module)
    const sourcePort = sourceModule && descriptorFor(sourceModule.type_id)?.outputs.find((port) => port.id === wire.source.port)
    const targetPort = targetModule && descriptorFor(targetModule.type_id)?.inputs.find((port) => port.id === wire.target.port)
    if (!sourceModule || !targetModule || !sourcePort || !targetPort) throw new Error(`Wire ${wire.id} has an unknown endpoint.`)
    const result = portsCompatible({ ...wire.source, direction: 'output', signal: sourcePort.signal } as PortRef, { ...wire.target, direction: 'input', signal: targetPort.signal } as PortRef)
    if (!result.valid || wire.signal !== sourcePort.signal) throw new Error(`Wire ${wire.id}: ${result.reason ?? 'signal mismatch'}`)
  }
}
