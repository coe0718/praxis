import { createContext, useContext, useEffect, useRef, useState } from 'react'
import type { PraxisEvent } from '../lib/api'

interface SSEContextValue {
  events: PraxisEvent[]
  connected: boolean
}

const SSEContext = createContext<SSEContextValue>({ events: [], connected: false })

const EVENT_TYPES = [
  'session.start',
  'session.end',
  'phase.orient.start',
  'phase.act.start',
  'phase.reflect.start',
  'agent:speculative_branch_selected',
  'agent:learning_opportunities_found',
  'agent:steered',
]

export function SSEProvider({ children }: { children: React.ReactNode }): React.ReactElement {
  const [events, setEvents] = useState<PraxisEvent[]>([])
  const [connected, setConnected] = useState(false)
  const esRef = useRef<EventSource | null>(null)

  useEffect(() => {
    const base = sessionStorage.getItem('praxis_base_url') ?? ''
    const token = sessionStorage.getItem('praxis_token')
    const url = token ? `${base}/events?token=${encodeURIComponent(token)}` : `${base}/events`
    const es = new EventSource(url)
    esRef.current = es

    es.addEventListener('open', () => setConnected(true))
    es.addEventListener('error', () => setConnected(false))

    es.addEventListener('message', (e) => {
      const event: PraxisEvent = { kind: e.type ?? 'message', detail: e.data, at: new Date().toISOString() }
      setEvents((prev) => [event, ...prev].slice(0, 100))
    })

    const handlers = EVENT_TYPES.map((type) => {
      const handler = (e: MessageEvent): void => {
        setEvents((prev) =>
          [{ kind: type, detail: e.data, at: new Date().toISOString() }, ...prev].slice(0, 100),
        )
      }
      es.addEventListener(type, handler as EventListener)
      return { type, handler: handler as EventListener }
    })

    return () => {
      handlers.forEach(({ type, handler }) => es.removeEventListener(type, handler))
      es.close()
      setConnected(false)
    }
  }, [])

  return <SSEContext.Provider value={{ events, connected }}>{children}</SSEContext.Provider>
}

export function useSSEContext(): SSEContextValue {
  return useContext(SSEContext)
}
