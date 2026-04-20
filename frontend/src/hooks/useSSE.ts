import { useSSEContext } from '../contexts/SSEContext'
import type { PraxisEvent } from '../lib/api'

export function useSSE(maxEvents = 50): { events: PraxisEvent[]; connected: boolean } {
  const { events, connected } = useSSEContext()
  return { events: events.slice(0, maxEvents), connected }
}
