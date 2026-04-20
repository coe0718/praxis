import { useEffect, useRef, useState } from 'react'
import type { PraxisEvent } from '../lib/api'

export function useSSE(maxEvents = 50) {
  const [events, setEvents] = useState<PraxisEvent[]>([])
  const [connected, setConnected] = useState(false)
  const esRef = useRef<EventSource | null>(null)

  useEffect(() => {
    const token = localStorage.getItem('praxis_token')
    const base = localStorage.getItem('praxis_base_url') ?? ''

    // SSE doesn't support custom headers; pass token as query param only if set
    const url = token
      ? `${base}/events?token=${encodeURIComponent(token)}`
      : `${base}/events`

    const es = new EventSource(url)
    esRef.current = es

    es.addEventListener('open', () => setConnected(true))
    es.addEventListener('error', () => setConnected(false))

    es.addEventListener('message', (e) => {
      const event: PraxisEvent = {
        kind: e.type ?? 'message',
        detail: e.data,
        at: new Date().toISOString(),
      }
      setEvents((prev) => [event, ...prev].slice(0, maxEvents))
    })

    // Catch all named events from the server
    const namedEventHandler = (type: string) => (e: MessageEvent) => {
      setEvents((prev) =>
        [{ kind: type, detail: e.data, at: new Date().toISOString() }, ...prev].slice(0, maxEvents),
      )
    }

    const eventTypes = [
      'session.start',
      'session.end',
      'phase.orient.start',
      'phase.act.start',
      'phase.reflect.start',
      'agent:speculative_branch_selected',
      'agent:learning_opportunities_found',
      'agent:steered',
    ]
    const handlers = eventTypes.map((t) => {
      const h = namedEventHandler(t)
      es.addEventListener(t, h as EventListener)
      return { type: t, handler: h as EventListener }
    })

    return () => {
      handlers.forEach(({ type, handler }) => es.removeEventListener(type, handler))
      es.close()
      setConnected(false)
    }
  }, [maxEvents])

  return { events, connected }
}
