import { useQuery } from '@tanstack/react-query'
import {
  Activity,
  Heart,
  Shield,
  TrendingUp,
  Zap,
  DollarSign,
} from 'lucide-react'
import {
  Area,
  AreaChart,
  Bar,
  BarChart,
  CartesianGrid,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from 'recharts'
import { StatCard, Card } from '../components/ui/Card'
import { Badge } from '../components/ui/Badge'
import { PageSpinner } from '../components/ui/Spinner'
import { Empty } from '../components/ui/Empty'
import { useSSE } from '../hooks/useSSE'
import {
  fetchSummary,
  fetchSessions,
  fetchApprovals,
  fetchScores,
  fetchHeartbeat,
  fetchTokenSummary,
  fetchTokenSessions,
  fetchHealth,
} from '../lib/api'
import { formatRelative, outcomeBadgeClass, formatDuration } from '../lib/utils'

export function Dashboard() {
  const { events, connected } = useSSE(30)

  const { data: summary } = useQuery({
    queryKey: ['summary'],
    queryFn: fetchSummary,
    refetchInterval: 30_000,
  })
  const { data: sessions, isLoading: loadingSessions } = useQuery({
    queryKey: ['sessions'],
    queryFn: fetchSessions,
    refetchInterval: 60_000,
  })
  const { data: approvals } = useQuery({
    queryKey: ['approvals'],
    queryFn: () => fetchApprovals(),
    refetchInterval: 30_000,
  })
  const { data: scoreRows } = useQuery({
    queryKey: ['score'],
    queryFn: fetchScores,
    refetchInterval: 60_000,
  })
  const { data: heartbeat } = useQuery({
    queryKey: ['heartbeat'],
    queryFn: fetchHeartbeat,
    refetchInterval: 15_000,
  })
  const { data: tokenSummary } = useQuery({
    queryKey: ['tokens'],
    queryFn: fetchTokenSummary,
    refetchInterval: 60_000,
  })
  const { data: tokenSessions } = useQuery({
    queryKey: ['tokenSessions'],
    queryFn: fetchTokenSessions,
    refetchInterval: 60_000,
  })
  const { data: health } = useQuery({
    queryKey: ['health'],
    queryFn: fetchHealth,
    refetchInterval: 30_000,
  })

  const pendingApprovals = approvals?.filter((a) => a.status === 'pending').length ?? 0
  const recentSessions = sessions?.slice(0, 5) ?? []
  const scoreData = (scoreRows ?? []).slice(-20).map((r) => ({
    composite: +r.composite.toFixed(3),
    anticipation: +r.anticipation.toFixed(3),
    follow_through: +r.follow_through.toFixed(3),
  }))

  const latestScore = scoreData[scoreData.length - 1]
  const hbPhase = (heartbeat?.phase as string) ?? '—'
  const hbUpdated = (heartbeat?.updated_at as string) ?? ''

  const tokenChartData = (tokenSessions ?? [])
    .slice()
    .reverse()
    .map((r) => ({
      session: `#${r.session_id}`,
      tokens: r.tokens_used,
      cost: r.estimated_cost_micros / 1_000_000,
    }))

  const healthStatus = health?.status ?? 'unknown'
  const healthColor =
    healthStatus === 'ok'
      ? 'text-emerald-500'
      : healthStatus === 'warn'
        ? 'text-amber-500'
        : 'text-red-500'

  return (
    <div className="space-y-6">
      <div className="page-header">
        <div>
          <h1 className="page-title">Dashboard</h1>
          <p className="page-subtitle">
            {hbPhase !== '—' ? `Agent phase: ${hbPhase}` : 'Praxis operational overview'}
            {hbUpdated && ` · updated ${formatRelative(hbUpdated)}`}
          </p>
        </div>
        {connected && (
          <div className="flex items-center gap-2 text-xs text-emerald-600 dark:text-emerald-400 font-medium">
            <span className="live-dot" />
            Live
          </div>
        )}
      </div>

      {/* Stat cards */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
        <StatCard
          label="Total Sessions"
          value={(summary?.session_count as number) ?? sessions?.length ?? '—'}
          icon={<Activity className="w-5 h-5" />}
        />
        <StatCard
          label="Pending Approvals"
          value={pendingApprovals}
          icon={<Shield className="w-5 h-5" />}
        />
        <StatCard
          label="Composite Score"
          value={latestScore?.composite ?? '—'}
          icon={<TrendingUp className="w-5 h-5" />}
        />
        <StatCard
          label="Agent Phase"
          value={<span className="text-base capitalize">{hbPhase}</span>}
          icon={<Heart className="w-5 h-5" />}
        />
      </div>

      {/* Token + Health stats */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
        <StatCard
          label="Total Tokens"
          value={tokenSummary?.total_tokens?.toLocaleString() ?? '—'}
          icon={<Zap className="w-5 h-5" />}
        />
        <StatCard
          label="Est. Cost"
          value={
            tokenSummary
              ? `$${(tokenSummary.total_cost_micros / 1_000_000).toFixed(2)}`
              : '—'
          }
          icon={<DollarSign className="w-5 h-5" />}
        />
        <StatCard
          label="Token Sessions"
          value={tokenSummary?.total_sessions ?? '—'}
          icon={<Activity className="w-5 h-5" />}
        />
        <StatCard
          label="Health"
          value={
            <span className={`text-base capitalize ${healthColor}`}>{healthStatus}</span>
          }
          icon={<Heart className={`w-5 h-5 ${healthColor}`} />}
        />
      </div>

      <div className="grid grid-cols-1 xl:grid-cols-3 gap-6">
        {/* Score chart */}
        <Card className="xl:col-span-2" padding="none">
          <div className="px-5 pt-5 pb-3 flex items-center justify-between border-b border-slate-100 dark:border-slate-800">
            <h2 className="text-sm font-semibold text-slate-700 dark:text-slate-300">
              Score Trend (last 20 sessions)
            </h2>
            <Badge variant="info">Composite</Badge>
          </div>
          <div className="p-5">
            {scoreData.length === 0 ? (
              <Empty
                icon={<TrendingUp className="w-8 h-8" />}
                title="No score data yet"
                description="Scores are computed at the end of each session."
              />
            ) : (
              <ResponsiveContainer width="100%" height={200}>
                <AreaChart data={scoreData} margin={{ top: 4, right: 4, left: -20, bottom: 0 }}>
                  <defs>
                    <linearGradient id="composite" x1="0" y1="0" x2="0" y2="1">
                      <stop offset="5%" stopColor="#6366f1" stopOpacity={0.3} />
                      <stop offset="95%" stopColor="#6366f1" stopOpacity={0} />
                    </linearGradient>
                  </defs>
                  <CartesianGrid strokeDasharray="3 3" stroke="#334155" strokeOpacity={0.3} />
                  <XAxis hide />
                  <YAxis domain={[0, 1]} tick={{ fontSize: 10, fill: '#94a3b8' }} />
                  <Tooltip
                    contentStyle={{
                      background: '#1e293b',
                      border: '1px solid #334155',
                      borderRadius: 8,
                      fontSize: 12,
                    }}
                    labelStyle={{ color: '#94a3b8' }}
                  />
                  <Area
                    type="monotone"
                    dataKey="composite"
                    stroke="#6366f1"
                    fill="url(#composite)"
                    strokeWidth={2}
                  />
                  <Area
                    type="monotone"
                    dataKey="follow_through"
                    stroke="#10b981"
                    fill="none"
                    strokeWidth={1.5}
                    strokeDasharray="4 2"
                  />
                </AreaChart>
              </ResponsiveContainer>
            )}
          </div>
        </Card>

        {/* Live event feed */}
        <Card padding="none">
          <div className="px-5 pt-5 pb-3 flex items-center justify-between border-b border-slate-100 dark:border-slate-800">
            <h2 className="text-sm font-semibold text-slate-700 dark:text-slate-300">
              Live Events
            </h2>
            <div className={`live-dot ${!connected ? 'bg-slate-400 animate-none' : ''}`} />
          </div>
          <div className="divide-y divide-slate-100 dark:divide-slate-800 max-h-72 overflow-y-auto">
            {events.length === 0 ? (
              <Empty
                icon={<Activity className="w-6 h-6" />}
                title="No events yet"
                description="Events appear here in real time."
              />
            ) : (
              events.map((ev, i) => (
                <div key={i} className="px-5 py-2.5">
                  <div className="flex items-center justify-between gap-2">
                    <span className="text-xs font-mono font-medium text-brand-600 dark:text-brand-400 truncate">
                      {ev.kind}
                    </span>
                    <span className="text-xs text-slate-400 flex-shrink-0">
                      {formatRelative(ev.at)}
                    </span>
                  </div>
                  {ev.detail && (
                    <p className="mt-0.5 text-xs text-slate-500 dark:text-slate-400 truncate">
                      {ev.detail}
                    </p>
                  )}
                </div>
              ))
            )}
          </div>
        </Card>
      </div>

      {/* Token usage chart */}
      <Card padding="none">
        <div className="px-5 pt-5 pb-3 flex items-center justify-between border-b border-slate-100 dark:border-slate-800">
          <h2 className="text-sm font-semibold text-slate-700 dark:text-slate-300">
            Token Usage by Session
          </h2>
          <Badge variant="info">Tokens</Badge>
        </div>
        <div className="p-5">
          {tokenChartData.length === 0 ? (
            <Empty
              icon={<Zap className="w-8 h-8" />}
              title="No token data yet"
              description="Token usage appears after sessions complete."
            />
          ) : (
            <ResponsiveContainer width="100%" height={200}>
              <BarChart data={tokenChartData} margin={{ top: 4, right: 4, left: -20, bottom: 0 }}>
                <CartesianGrid strokeDasharray="3 3" stroke="#334155" strokeOpacity={0.3} />
                <XAxis dataKey="session" tick={{ fontSize: 10, fill: '#94a3b8' }} />
                <YAxis tick={{ fontSize: 10, fill: '#94a3b8' }} />
                <Tooltip
                  contentStyle={{
                    background: '#1e293b',
                    border: '1px solid #334155',
                    borderRadius: 8,
                    fontSize: 12,
                  }}
                  labelStyle={{ color: '#94a3b8' }}
                />
                <Bar dataKey="tokens" fill="#6366f1" radius={[4, 4, 0, 0]} />
              </BarChart>
            </ResponsiveContainer>
          )}
        </div>
      </Card>

      {/* Recent sessions */}
      <Card padding="none">
        <div className="px-5 pt-5 pb-3 border-b border-slate-100 dark:border-slate-800">
          <h2 className="text-sm font-semibold text-slate-700 dark:text-slate-300">
            Recent Sessions
          </h2>
        </div>
        {loadingSessions ? (
          <PageSpinner />
        ) : recentSessions.length === 0 ? (
          <Empty
            icon={<Activity className="w-8 h-8" />}
            title="No sessions yet"
            description="Run praxis to start a session."
          />
        ) : (
          <div className="divide-y divide-slate-100 dark:divide-slate-800">
            {recentSessions.map((s) => (
              <div key={s.id} className="px-5 py-3 flex items-center gap-4">
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <span className={outcomeBadgeClass(s.outcome)}>{s.outcome}</span>
                    {s.selected_goal_title && (
                      <span className="text-xs text-slate-500 dark:text-slate-400 truncate">
                        {s.selected_goal_title}
                      </span>
                    )}
                  </div>
                  <p className="mt-0.5 text-xs text-slate-500 dark:text-slate-400 truncate">
                    {s.action_summary}
                  </p>
                </div>
                <div className="flex-shrink-0 text-right">
                  <p className="text-xs font-mono text-slate-500">
                    {formatDuration(s.started_at, s.ended_at)}
                  </p>
                  <p className="text-xs text-slate-400">{formatRelative(s.started_at)}</p>
                </div>
              </div>
            ))}
          </div>
        )}
      </Card>
    </div>
  )
}
