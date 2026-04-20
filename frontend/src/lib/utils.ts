export function cn(...classes: (string | undefined | null | false)[]): string {
  return classes.filter(Boolean).join(' ')
}

export function formatDate(iso: string): string {
  try {
    return new Date(iso).toLocaleString(undefined, {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    })
  } catch {
    return iso
  }
}

export function formatDateShort(iso: string): string {
  try {
    return new Date(iso).toLocaleDateString(undefined, {
      month: 'short',
      day: 'numeric',
    })
  } catch {
    return iso
  }
}

export function formatRelative(iso: string): string {
  try {
    const ms = Date.now() - new Date(iso).getTime()
    const s = Math.floor(ms / 1000)
    if (s < 60) return `${s}s ago`
    const m = Math.floor(s / 60)
    if (m < 60) return `${m}m ago`
    const h = Math.floor(m / 60)
    if (h < 24) return `${h}h ago`
    const d = Math.floor(h / 24)
    return `${d}d ago`
  } catch {
    return iso
  }
}

export function formatDuration(startIso: string, endIso: string): string {
  try {
    const ms = new Date(endIso).getTime() - new Date(startIso).getTime()
    const s = Math.floor(ms / 1000)
    if (s < 60) return `${s}s`
    const m = Math.floor(s / 60)
    if (m < 60) return `${m}m ${s % 60}s`
    return `${Math.floor(m / 60)}h ${m % 60}m`
  } catch {
    return ''
  }
}

export function outcomeColor(outcome: string): string {
  if (outcome.includes('completed') || outcome === 'success') return 'text-emerald-500'
  if (outcome.includes('fail') || outcome.includes('error')) return 'text-red-400'
  if (outcome === 'idle') return 'text-slate-400'
  if (outcome === 'delegated') return 'text-blue-400'
  if (outcome === 'steered') return 'text-violet-400'
  return 'text-amber-400'
}

export function outcomeBadgeClass(outcome: string): string {
  if (outcome.includes('completed') || outcome === 'success')
    return 'badge bg-emerald-100 text-emerald-700 dark:bg-emerald-900/40 dark:text-emerald-400'
  if (outcome.includes('fail') || outcome.includes('error'))
    return 'badge bg-red-100 text-red-700 dark:bg-red-900/40 dark:text-red-400'
  if (outcome === 'idle')
    return 'badge bg-slate-100 text-slate-600 dark:bg-slate-800 dark:text-slate-400'
  if (outcome === 'delegated')
    return 'badge bg-blue-100 text-blue-700 dark:bg-blue-900/40 dark:text-blue-400'
  return 'badge bg-amber-100 text-amber-700 dark:bg-amber-900/40 dark:text-amber-400'
}

export function statusBadgeClass(status: string): string {
  switch (status) {
    case 'approved':
      return 'badge bg-emerald-100 text-emerald-700 dark:bg-emerald-900/40 dark:text-emerald-400'
    case 'pending':
      return 'badge bg-amber-100 text-amber-700 dark:bg-amber-900/40 dark:text-amber-400'
    case 'rejected':
      return 'badge bg-red-100 text-red-700 dark:bg-red-900/40 dark:text-red-400'
    case 'executed':
      return 'badge bg-blue-100 text-blue-700 dark:bg-blue-900/40 dark:text-blue-400'
    case 'claiming':
      return 'badge bg-violet-100 text-violet-700 dark:bg-violet-900/40 dark:text-violet-400'
    default:
      return 'badge bg-slate-100 text-slate-600 dark:bg-slate-800 dark:text-slate-400'
  }
}

export function truncate(s: string, max: number): string {
  return s.length > max ? s.slice(0, max) + '…' : s
}
