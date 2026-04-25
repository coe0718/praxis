import { useEffect } from 'react'
import { useNavigate } from 'react-router-dom'

const KEY_MAP: Record<string, string> = {
  g: '/goals',
  a: '/approvals',
  r: '/sessions',
  w: '/dashboard',
  t: '/tools',
  m: '/memories',
  s: '/score',
  c: '/chat',
  e: '/evolution',
  b: '/boundaries',
  v: '/vault',
  d: '/delegation',
  l: '/learning',
  f: '/forensics',
  i: '/identity',
  n: '/canary',
  o: '/config',
  '?': '/help',
}

export function useKeyboardShortcuts(enabled = true): void {
  const navigate = useNavigate()

  useEffect(() => {
    if (!enabled) return

    const handler = (e: KeyboardEvent) => {
      // Ignore when typing in inputs, textareas, or contenteditable
      const target = e.target as HTMLElement
      if (
        target.tagName === 'INPUT' ||
        target.tagName === 'TEXTAREA' ||
        target.isContentEditable
      ) {
        // Allow Escape to close modals even in inputs
        if (e.key === 'Escape') {
          const modals = document.querySelectorAll('[data-modal]')
          if (modals.length > 0) {
            e.preventDefault()
            modals[modals.length - 1].dispatchEvent(new Event('close'))
          }
        }
        return
      }

      // Slash for search focus
      if (e.key === '/') {
        e.preventDefault()
        const searchInput = document.querySelector<HTMLInputElement>('input[type="text"][placeholder*="Search"]')
        searchInput?.focus()
        return
      }

      // Escape to close modals
      if (e.key === 'Escape') {
        const modals = document.querySelectorAll('[data-modal]')
        if (modals.length > 0) {
          e.preventDefault()
          modals[modals.length - 1].dispatchEvent(new Event('close'))
        }
        return
      }

      // Navigation shortcuts
      const path = KEY_MAP[e.key.toLowerCase()]
      if (path && !e.ctrlKey && !e.metaKey && !e.altKey) {
        e.preventDefault()
        navigate(path)
      }
    }

    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [enabled, navigate])
}
