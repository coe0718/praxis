import { useQuery } from '@tanstack/react-query'
import {
  AreaChart, Area, XAxis, YAxis, CartesianGrid, Tooltip, Legend,
  ResponsiveContainer, RadarChart, PolarGrid, PolarAngleAxis, Radar,
} from 'recharts'
import { TrendingUp } from 'lucide-react'
import { fetchScores, type SessionScore } from '../lib/api'
import { Card, StatCard } from '../components/ui/Card'
import { PageSpinner } from '../components/ui/Spinner'
import { Empty } from '../components/ui/Empty'
import { formatDateShort } from '../lib/utils'

function avg(scores: SessionScore[], key: keyof SessionScore): number {
  if (scores.length === 0) return 0
  const vals = scores.map((s) => (s[key] as number) ?? 0)
  return vals.reduce((a, b) => a + b, 0) / vals.length
}

export function Score() {
  const { data: scores = [], isLoading } = useQuery({
    queryKey: ['scores'],
    queryFn: fetchScores,
    refetchInterval: 60_000,
  })

  const recent = scores.slice(-30)
  const last = scores[scores.length - 1]

  const chartData = recent.map((s) => ({
    label: formatDateShort(s.created_at),
    composite: +(s.composite * 100).toFixed(1),
    anticipation: +(s.anticipation * 100).toFixed(1),
    follow_through: +(s.follow_through * 100).toFixed(1),
    reliability: +(s.reliability * 100).toFixed(1),
    independence: +(s.independence * 100).toFixed(1),
  }))

  const radarData = last
    ? [
        { dimension: 'Anticipation', value: +(last.anticipation * 100).toFixed(1) },
        { dimension: 'Follow-Through', value: +(last.follow_through * 100).toFixed(1) },
        { dimension: 'Reliability', value: +(last.reliability * 100).toFixed(1) },
        { dimension: 'Independence', value: +(last.independence * 100).toFixed(1) },
      ]
    : []

  const tooltipStyle = {
    backgroundColor: 'var(--tooltip-bg, #1e293b)',
    border: 'none',
    borderRadius: '8px',
    fontSize: '12px',
  }

  return (
    <div className="space-y-6">
      <div className="page-header">
        <div>
          <h1 className="page-title">Score</h1>
          <p className="page-subtitle">{scores.length} sessions scored</p>
        </div>
      </div>

      {isLoading ? (
        <PageSpinner />
      ) : scores.length === 0 ? (
        <Empty
          icon={<TrendingUp className="w-8 h-8" />}
          title="No scores yet"
          description="Session scores are computed after each reflect phase."
        />
      ) : (
        <div className="space-y-6">
          {/* Summary stats */}
          <div className="grid grid-cols-2 sm:grid-cols-5 gap-4">
            <StatCard
              label="Composite (avg)"
              value={(avg(scores, 'composite') * 100).toFixed(1) + '%'}
            />
            <StatCard
              label="Anticipation"
              value={(avg(scores, 'anticipation') * 100).toFixed(1) + '%'}
            />
            <StatCard
              label="Follow-Through"
              value={(avg(scores, 'follow_through') * 100).toFixed(1) + '%'}
            />
            <StatCard
              label="Reliability"
              value={(avg(scores, 'reliability') * 100).toFixed(1) + '%'}
            />
            <StatCard
              label="Independence"
              value={(avg(scores, 'independence') * 100).toFixed(1) + '%'}
            />
          </div>

          <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
            {/* Trend chart */}
            <Card className="lg:col-span-2" padding="none">
              <div className="px-5 py-4 border-b border-slate-100 dark:border-slate-800">
                <h2 className="text-sm font-semibold text-slate-700 dark:text-slate-300">Score Trend (last 30 sessions)</h2>
              </div>
              <div className="p-5">
                <ResponsiveContainer width="100%" height={260}>
                  <AreaChart data={chartData}>
                    <defs>
                      <linearGradient id="composite" x1="0" y1="0" x2="0" y2="1">
                        <stop offset="5%" stopColor="#6366f1" stopOpacity={0.3} />
                        <stop offset="95%" stopColor="#6366f1" stopOpacity={0} />
                      </linearGradient>
                    </defs>
                    <CartesianGrid strokeDasharray="3 3" stroke="rgba(148,163,184,0.15)" />
                    <XAxis dataKey="label" tick={{ fontSize: 10, fill: '#94a3b8' }} tickLine={false} />
                    <YAxis domain={[0, 100]} tick={{ fontSize: 10, fill: '#94a3b8' }} tickLine={false} axisLine={false} />
                    <Tooltip contentStyle={tooltipStyle} formatter={(v: number) => `${v}%`} />
                    <Legend wrapperStyle={{ fontSize: '11px' }} />
                    <Area type="monotone" dataKey="composite" name="Composite" stroke="#6366f1" fill="url(#composite)" strokeWidth={2} dot={false} />
                    <Area type="monotone" dataKey="anticipation" name="Anticipation" stroke="#f59e0b" fill="none" strokeWidth={1.5} dot={false} strokeDasharray="4 2" />
                    <Area type="monotone" dataKey="follow_through" name="Follow-Through" stroke="#10b981" fill="none" strokeWidth={1.5} dot={false} strokeDasharray="4 2" />
                    <Area type="monotone" dataKey="reliability" name="Reliability" stroke="#3b82f6" fill="none" strokeWidth={1.5} dot={false} strokeDasharray="4 2" />
                    <Area type="monotone" dataKey="independence" name="Independence" stroke="#ec4899" fill="none" strokeWidth={1.5} dot={false} strokeDasharray="4 2" />
                  </AreaChart>
                </ResponsiveContainer>
              </div>
            </Card>

            {/* Radar — latest session */}
            {radarData.length > 0 && (
              <Card padding="none">
                <div className="px-5 py-4 border-b border-slate-100 dark:border-slate-800">
                  <h2 className="text-sm font-semibold text-slate-700 dark:text-slate-300">Latest Session</h2>
                </div>
                <div className="p-5">
                  <ResponsiveContainer width="100%" height={220}>
                    <RadarChart data={radarData}>
                      <PolarGrid stroke="rgba(148,163,184,0.2)" />
                      <PolarAngleAxis dataKey="dimension" tick={{ fontSize: 10, fill: '#94a3b8' }} />
                      <Radar name="Score" dataKey="value" stroke="#6366f1" fill="#6366f1" fillOpacity={0.25} />
                    </RadarChart>
                  </ResponsiveContainer>
                  <div className="mt-2 text-center">
                    <p className="text-2xl font-bold text-brand-600 dark:text-brand-400">
                      {(last!.composite * 100).toFixed(1)}%
                    </p>
                    <p className="text-xs text-slate-400">composite score</p>
                  </div>
                </div>
              </Card>
            )}
          </div>

          {/* Recent sessions table */}
          <Card padding="none">
            <div className="px-5 py-4 border-b border-slate-100 dark:border-slate-800">
              <h2 className="text-sm font-semibold text-slate-700 dark:text-slate-300">All Sessions</h2>
            </div>
            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b border-slate-100 dark:border-slate-800">
                    {['Session', 'Composite', 'Anticipation', 'Follow-Through', 'Reliability', 'Independence'].map((h) => (
                      <th key={h} className="px-5 py-2.5 text-left text-xs font-medium text-slate-400 uppercase tracking-wider">
                        {h}
                      </th>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  {scores.slice().reverse().map((s) => (
                    <tr key={s.session_id} className="border-b border-slate-50 dark:border-slate-800/50 hover:bg-slate-50 dark:hover:bg-slate-800/30 transition-colors">
                      <td className="px-5 py-2.5 font-mono text-slate-700 dark:text-slate-300">#{s.session_id}</td>
                      <td className="px-5 py-2.5 font-semibold text-brand-600 dark:text-brand-400">{(s.composite * 100).toFixed(1)}%</td>
                      <td className="px-5 py-2.5 text-slate-600 dark:text-slate-400">{(s.anticipation * 100).toFixed(1)}%</td>
                      <td className="px-5 py-2.5 text-slate-600 dark:text-slate-400">{(s.follow_through * 100).toFixed(1)}%</td>
                      <td className="px-5 py-2.5 text-slate-600 dark:text-slate-400">{(s.reliability * 100).toFixed(1)}%</td>
                      <td className="px-5 py-2.5 text-slate-600 dark:text-slate-400">{(s.independence * 100).toFixed(1)}%</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </Card>
        </div>
      )}
    </div>
  )
}
