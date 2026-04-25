import { useQuery } from '@tanstack/react-query'
import { Zap, DollarSign, BarChart3, PieChart } from 'lucide-react'
import {
  Bar,
  BarChart,
  CartesianGrid,
  Cell,
  Pie,
  PieChart as RePieChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from 'recharts'
import { Card } from '../components/ui/Card'
import { PageSpinner } from '../components/ui/Spinner'
import { Empty } from '../components/ui/Empty'
import { fetchTokenSummary, fetchTokenSessions } from '../lib/api'

const COLORS = ['#6366f1', '#10b981', '#f59e0b', '#ef4444', '#8b5cf6', '#ec4899']

export function Tokens() {
  const { data: summary, isLoading: loadingSummary } = useQuery({
    queryKey: ['tokens'],
    queryFn: fetchTokenSummary,
    refetchInterval: 60_000,
  })
  const { data: sessions, isLoading: loadingSessions } = useQuery({
    queryKey: ['tokenSessions'],
    queryFn: fetchTokenSessions,
    refetchInterval: 60_000,
  })

  const chartData = (sessions ?? [])
    .slice()
    .reverse()
    .map((r) => ({
      session: `#${r.session_id}`,
      tokens: r.tokens_used,
      cost: +(r.estimated_cost_micros / 1_000_000).toFixed(2),
    }))

  const providerData = (summary?.by_provider ?? []).map((p) => ({
    name: p.provider,
    tokens: p.tokens_used,
    cost: +(p.estimated_cost_micros / 1_000_000).toFixed(2),
  }))

  return (
    <div className="space-y-6">
      <div className="page-header">
        <div>
          <h1 className="page-title">Token Spend</h1>
          <p className="page-subtitle">
            Track token usage and estimated cost across providers.
          </p>
        </div>
      </div>

      {/* Summary stats */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
        <Card>
          <div className="flex items-center gap-3">
            <Zap className="w-5 h-5 text-brand-500" />
            <div>
              <p className="text-xs text-slate-500 dark:text-slate-400">Total Tokens</p>
              <p className="text-xl font-semibold text-slate-800 dark:text-slate-200">
                {summary?.total_tokens?.toLocaleString() ?? '—'}
              </p>
            </div>
          </div>
        </Card>
        <Card>
          <div className="flex items-center gap-3">
            <DollarSign className="w-5 h-5 text-emerald-500" />
            <div>
              <p className="text-xs text-slate-500 dark:text-slate-400">Est. Cost</p>
              <p className="text-xl font-semibold text-slate-800 dark:text-slate-200">
                {summary ? `$${(summary.total_cost_micros / 1_000_000).toFixed(2)}` : '—'}
              </p>
            </div>
          </div>
        </Card>
        <Card>
          <div className="flex items-center gap-3">
            <BarChart3 className="w-5 h-5 text-amber-500" />
            <div>
              <p className="text-xs text-slate-500 dark:text-slate-400">Sessions Tracked</p>
              <p className="text-xl font-semibold text-slate-800 dark:text-slate-200">
                {summary?.total_sessions ?? '—'}
              </p>
            </div>
          </div>
        </Card>
        <Card>
          <div className="flex items-center gap-3">
            <PieChart className="w-5 h-5 text-violet-500" />
            <div>
              <p className="text-xs text-slate-500 dark:text-slate-400">Providers</p>
              <p className="text-xl font-semibold text-slate-800 dark:text-slate-200">
                {summary?.by_provider?.length ?? '—'}
              </p>
            </div>
          </div>
        </Card>
      </div>

      <div className="grid grid-cols-1 xl:grid-cols-2 gap-6">
        {/* Tokens by session */}
        <Card padding="none">
          <div className="px-5 pt-5 pb-3 border-b border-slate-100 dark:border-slate-800">
            <h2 className="text-sm font-semibold text-slate-700 dark:text-slate-300">
              Tokens per Session
            </h2>
          </div>
          <div className="p-5">
            {loadingSessions ? (
              <PageSpinner />
            ) : chartData.length === 0 ? (
              <Empty
                icon={<Zap className="w-8 h-8" />}
                title="No token data yet"
                description="Token usage appears after sessions complete."
              />
            ) : (
              <ResponsiveContainer width="100%" height={250}>
                <BarChart data={chartData} margin={{ top: 4, right: 4, left: -20, bottom: 0 }}>
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

        {/* Provider breakdown */}
        <Card padding="none">
          <div className="px-5 pt-5 pb-3 border-b border-slate-100 dark:border-slate-800">
            <h2 className="text-sm font-semibold text-slate-700 dark:text-slate-300">
              Provider Breakdown
            </h2>
          </div>
          <div className="p-5">
            {loadingSummary ? (
              <PageSpinner />
            ) : providerData.length === 0 ? (
              <Empty
                icon={<PieChart className="w-8 h-8" />}
                title="No provider data yet"
                description="Provider usage appears after remote sessions."
              />
            ) : (
              <div className="flex flex-col md:flex-row items-center gap-6">
                <ResponsiveContainer width="100%" height={220}>
                  <RePieChart>
                    <Pie
                      data={providerData}
                      dataKey="tokens"
                      nameKey="name"
                      cx="50%"
                      cy="50%"
                      outerRadius={80}
                      label={({ name, percent }) =>
                        `${name}: ${(percent * 100).toFixed(0)}%`
                      }
                    >
                      {providerData.map((_, i) => (
                        <Cell key={i} fill={COLORS[i % COLORS.length]} />
                      ))}
                    </Pie>
                    <Tooltip
                      contentStyle={{
                        background: '#1e293b',
                        border: '1px solid #334155',
                        borderRadius: 8,
                        fontSize: 12,
                      }}
                    />
                  </RePieChart>
                </ResponsiveContainer>
                <div className="w-full md:w-auto space-y-2">
                  {providerData.map((p, i) => (
                    <div key={p.name} className="flex items-center gap-2 text-sm">
                      <span
                        className="w-3 h-3 rounded-full flex-shrink-0"
                        style={{ backgroundColor: COLORS[i % COLORS.length] }}
                      />
                      <span className="text-slate-700 dark:text-slate-300">{p.name}</span>
                      <span className="text-slate-400 ml-auto">
                        {p.tokens.toLocaleString()} tok · ${p.cost.toFixed(2)}
                      </span>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>
        </Card>
      </div>
    </div>
  )
}
