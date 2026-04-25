import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { ChevronDown, ChevronRight, Layers } from 'lucide-react'
import { fetchSessions, type SessionRow } from '../lib/api'
import { Card } from '../components/ui/Card'
import { PageSpinner } from '../components/ui/Spinner'
import { Empty } from '../components/ui/Empty'
import { SessionTimeline } from '../components/SessionTimeline'
import { formatDate, formatDuration, formatRelative, outcomeBadgeClass } from '../lib/utils'

function SessionCard({ s }: { s: SessionRow }) {
  const [expanded, setExpanded] = useState(false)

  return (
    <Card padding="none" className="overflow-hidden">
      <div
        className="px-5 py-3.5 flex items-center gap-4 cursor-pointer hover:bg-slate-50 dark:hover:bg-slate-800/50 transition-colors"
        onClick={() => setExpanded((e) => !e)}
      >
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 flex-wrap">
            <span className={outcomeBadgeClass(s.outcome)}>{s.outcome}</span>
            {s.selected_goal_title && (
              <span className="text-sm font-medium text-slate-700 dark:text-slate-300 truncate">
                {s.selected_goal_title}
              </span>
            )}
            {s.selected_task && !s.selected_goal_title && (
              <span className="text-sm text-slate-600 dark:text-slate-400 truncate">
                {s.selected_task}
              </span>
            )}
          </div>
          <p className="mt-0.5 text-xs text-slate-500 dark:text-slate-400 truncate">
            {s.action_summary}
          </p>
        </div>
        <div className="flex-shrink-0 flex items-center gap-3 text-right">
          <div>
            <p className="text-xs font-mono text-slate-500">
              {formatDuration(s.started_at, s.ended_at)}
            </p>
            <p className="text-xs text-slate-400">{formatRelative(s.started_at)}</p>
          </div>
          {expanded ? (
            <ChevronDown className="w-4 h-4 text-slate-400" />
          ) : (
            <ChevronRight className="w-4 h-4 text-slate-400" />
          )}
        </div>
      </div>

      {expanded && (
        <div className="px-5 pb-4 border-t border-slate-100 dark:border-slate-800 pt-3 space-y-3">
          <SessionTimeline durations={s.phase_durations} />
          <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 text-sm">
            <div>
              <p className="text-xs text-slate-400 uppercase tracking-wider">Session</p>
              <p className="font-mono text-slate-700 dark:text-slate-300">#{s.id}</p>
            </div>
            <div>
              <p className="text-xs text-slate-400 uppercase tracking-wider">Day</p>
              <p className="font-mono text-slate-700 dark:text-slate-300">{s.day}</p>
            </div>
            <div>
              <p className="text-xs text-slate-400 uppercase tracking-wider">Started</p>
              <p className="text-slate-700 dark:text-slate-300">{formatDate(s.started_at)}</p>
            </div>
            <div>
              <p className="text-xs text-slate-400 uppercase tracking-wider">Ended</p>
              <p className="text-slate-700 dark:text-slate-300">{formatDate(s.ended_at)}</p>
            </div>
          </div>
          {s.selected_goal_id && (
            <div>
              <p className="text-xs text-slate-400 uppercase tracking-wider mb-1">Goal</p>
              <p className="mono text-slate-600 dark:text-slate-300">{s.selected_goal_id}</p>
            </div>
          )}
          <div>
            <p className="text-xs text-slate-400 uppercase tracking-wider mb-1">Summary</p>
            <p className="text-sm text-slate-700 dark:text-slate-300 whitespace-pre-wrap">
              {s.action_summary}
            </p>
          </div>
        </div>
      )}
    </Card>
  )
}

export function Sessions() {
  const { data: sessions = [], isLoading } = useQuery({
    queryKey: ['sessions'],
    queryFn: fetchSessions,
    refetchInterval: 60_000,
  })

  return (
    <div className="space-y-6">
      <div className="page-header">
        <div>
          <h1 className="page-title">Sessions</h1>
          <p className="page-subtitle">{sessions.length} sessions recorded</p>
        </div>
      </div>

      {isLoading ? (
        <PageSpinner />
      ) : sessions.length === 0 ? (
        <Empty
          icon={<Layers className="w-8 h-8" />}
          title="No sessions yet"
          description="Sessions are created when the agent runs."
        />
      ) : (
        <div className="space-y-2">
          {sessions.map((s) => (
            <SessionCard key={s.id} s={s} />
          ))}
        </div>
      )}
    </div>
  )
}
