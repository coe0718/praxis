interface SessionTimelineProps {
  durations?: Record<string, number>
  className?: string
}

const PHASE_ORDER = ['orient', 'decide', 'act', 'reflect']

const PHASE_COLORS: Record<string, string> = {
  orient: 'bg-blue-400',
  decide: 'bg-violet-400',
  act: 'bg-emerald-400',
  reflect: 'bg-amber-400',
  sleep: 'bg-slate-300 dark:bg-slate-600',
}

const PHASE_LABELS: Record<string, string> = {
  orient: 'Orient',
  decide: 'Decide',
  act: 'Act',
  reflect: 'Reflect',
  sleep: 'Sleep',
}

export function SessionTimeline({ durations, className = '' }: SessionTimelineProps) {
  if (!durations || Object.keys(durations).length === 0) {
    return null
  }

  const total = Object.values(durations).reduce((sum, v) => sum + (v || 0), 0)
  if (total <= 0) {
    return (
      <div className={`flex items-center gap-2 text-xs text-slate-400 ${className}`}>
        <span>No timing data</span>
      </div>
    )
  }

  return (
    <div className={`space-y-1 ${className}`}>
      <div className="flex h-2 rounded-full overflow-hidden bg-slate-100 dark:bg-slate-800">
        {PHASE_ORDER.map((phase) => {
          const value = durations[phase] || 0
          const pct = total > 0 ? (value / total) * 100 : 0
          if (pct <= 0) return null
          return (
            <div
              key={phase}
              className={`${PHASE_COLORS[phase] || 'bg-slate-400'} transition-all`}
              style={{ width: `${pct}%` }}
              title={`${PHASE_LABELS[phase] || phase}: ${value.toFixed(1)}s`}
            />
          )
        })}
      </div>
      <div className="flex gap-3 text-[10px] text-slate-500 dark:text-slate-400">
        {PHASE_ORDER.map((phase) => {
          const value = durations[phase] || 0
          if (value <= 0) return null
          return (
            <div key={phase} className="flex items-center gap-1">
              <span className={`w-1.5 h-1.5 rounded-full ${PHASE_COLORS[phase] || 'bg-slate-400'}`} />
              <span>
                {PHASE_LABELS[phase] || phase} {value.toFixed(1)}s
              </span>
            </div>
          )
        })}
      </div>
    </div>
  )
}
