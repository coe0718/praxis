import { createContext, useContext, useEffect, useRef, useState, useCallback } from 'react'
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

function requestNotificationPermission(): void {
  if ('Notification' in window && Notification.permission === 'default') {
    Notification.requestPermission().catch(() => {})
  }
}

function sendBrowserNotification(title: string, body: string): void {
  if ('Notification' in window && Notification.permission === 'granted') {
    try {
      new Notification(title, { body, icon: '/favicon.ico' })
    } catch {
      // Ignore notification errors
    }
  }
}

export function SSEProvider({ children }: { children: React.ReactNode }): React.ReactElement {
  const [events, setEvents] = useState<PraxisEvent[]>([])
  const [connected, setConnected] = useState(false)
  const esRef = useRef<EventSource | null>(null)
  const lastApprovalCount = useRef(0)

  const handleApprovalEvent = useCallback((event: PraxisEvent) => {
    // Try to parse detail as JSON to extract approval count
    try {
      const data = JSON.parse(event.detail)
      const count = data.pending_count ?? data.count ?? 0
      if (typeof count === 'number' && count > lastApprovalCount.current) {
        sendBrowserNotification(
          'Praxis Approval Request',
          `${count} approval(s) pending — click to review`,
        )
      }
      lastApprovalCount.current = count
    } catch {
      // Fallback: any approval-related event text triggers a notification once
      if (event.detail.toLowerCase().includes('approval') && lastApprovalCount.current === 0) {
        sendBrowserNotification('Praxis Approval Request', event.detail)
        lastApprovalCount.current = 1
      }
    }
  }, [])

  useEffect(() => {
    requestNotificationPermission()

    const base = localStorage.getItem('praxis_base_url') ?? sessionStorage.getItem('praxis_base_url') ?? ''
    const token = localStorage.getItem('praxis_token') ?? sessionStorage.getItem('praxis_token')
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
        const event: PraxisEvent = { kind: type, detail: e.data, at: new Date().toISOString() }
        setEvents((prev) => [event, ...prev].slice(0, 100))

        // Browser notification for approval events
        if (type.includes('approval') || type.includes('tool')) {
          handleApprovalEvent(event)
        }
      }
      es.addEventListener(type, handler as EventListener)
      return { type, handler: handler as EventListener }
    })

    return () => {
      handlers.forEach(({ type, handler }) => es.removeEventListener(type, handler))
      es.close()
      setConnected(false)
    }
  }, [handleApprovalEvent])

  return <SSEContext.Provider value={{ events, connected }}>{children}</SSEContext.Provider>
}

export function useSSEContext(): SSEContextValue {
  return useContext(SSEContext)
}
