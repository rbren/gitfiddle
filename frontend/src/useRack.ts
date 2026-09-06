import { useSyncExternalStore } from 'react'
import type { RackStore } from './store'

export function useRack(store: RackStore) {
  return useSyncExternalStore(store.subscribe, store.getSnapshot, store.getSnapshot)
}
