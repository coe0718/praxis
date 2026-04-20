import { useQuery } from '@tanstack/react-query'
import { BarChart2, AlertCircle, TrendingDown, TrendingUp, Minus } from 'lucide-react'
import { fetchArgus, type ArgusReport } from '../lib/api'
import { Card, StatCard } from '../components/ui/Card'
import { Badge } from '../components/ui/Badge'
import { PageSpinner } from '../components/ui/Spinner'
import { Empty } from '../components/ui/Empty'

const driftIcon = (status: string) => {
  if (status === 'improving') return <TrendingUp className="w-4 h-4 text-emerald-500" />
  if (status === 'degrading') return <TrendingDown className="w-4 h-4 text-red-500" />
  return <Minus className="w-4 h-4 text-slate-400" />
}

const driftVariant = (status: string): React.ComponentProps<typeof Badge>['variant'] => {
  if (status === 'improving') return 'success'
  if (status === 'degrading') return 'danger'
  return 'default'
}

function FailureList({ title, items }: { title: string; items: string[] }) {
  if (items.length === 0) return null

  return (
    <Card padding="none">
      <div className="px-5 py-4 border-b border-slate-100 dark:border-slate-800 flex items-center gap-2">
        <AlertCircle className="w-4 h-4 text-red-400" />
        <h2 className="text-sm font-semibold text-slate-700 dark:text-slate-300">{title}</h2>
        <span className="ml-auto badge bg-red-100 dark:bg-red-900/30 text-red-600 dark:text-red-400">
          {items.length}
        </span>
      </div>
      <ul className="divide-y divide-slate-100 dark:divide-slate-800">
        {items.map((item, i) => (
          <li key={i} className="px-5 py-3 text-sm text-slate-600 dark:text-slate-400">
            {item}
          </li>
        ))}
      </ul>
    </Card>
  )
}

function RepeatedWorkSection({ report }: { report: ArgusReport }) {
  const items = report.repeated_work ?? []
  if (items.length === 0) return null

  return (
    <Card padding="none">
      <div className="px-5 py-4 border-b border-slate-100 dark:border-slate-800">
        <h2 className="text-sm font-semibold text-slate-700 dark:text-slate-300">
          Repeated Work ({items.length})
        </h2>
      </div>
      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-slate-100 dark:border-slate-800">
              {['Label', 'Sessions', 'Distinct Days', 'Latest Outcome'].map((h) => (
                <th key={h} className="px-5 py-2.5 text-left text-xs font-medium text-slate-400 uppercase tracking-wider">
                  {h}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {items.map((item, i) => (
              <tr key={i} className="border-b border-slate-50 dark:border-slate-800/50 hover:bg-slate-50 dark:hover:bg-slate-800/30 transition-colors">
                <td className="px-5 py-2.5 text-slate-700 dark:text-slate-300">{item.label}</td>
                <td className="px-5 py-2.5 font-mono text-slate-600 dark:text-slate-400">{item.sessions}</td>
                <td className="px-5 py-2.5 font-mono text-slate-600 dark:text-slate-400">{item.distinct_days}</td>
                <td className="px-5 py-2.5 text-slate-500 dark:text-slate-400">{item.latest_outcome}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </Card>
  )
}

function TokenHotspots({ report }: { report: ArgusReport }) {
  const items = report.token_hotspots ?? []
  if (items.length === 0) return null

  return (
    <Card padding="none">
      <div className="px-5 py-4 border-b border-slate-100 dark:border-slate-800">
        <h2 className="text-sm font-semibold text-slate-700 dark:text-slate-300">
          Token Hotspots
        </h2>
      </div>
      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-slate-100 dark:border-slate-800">
              {['Provider', 'Model', 'Tokens'].map((h) => (
                <th key={h} className="px-5 py-2.5 text-left text-xs font-medium text-slate-400 uppercase tracking-wider">
                  {h}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {items.map((item, i) => (
              <tr key={i} className="border-b border-slate-50 dark:border-slate-800/50 hover:bg-slate-50 dark:hover:bg-slate-800/30 transition-colors">
                <td className="px-5 py-2.5 font-mono text-slate-700 dark:text-slate-300">{item.provider}</td>
                <td className="px-5 py-2.5 font-mono text-slate-600 dark:text-slate-400">{item.model}</td>
                <td className="px-5 py-2.5 font-mono text-brand-600 dark:text-brand-400">
                  {item.tokens.toLocaleString()}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </Card>
  )
}

export function Argus() {
  const { data: report, isLoading } = useQuery({
    queryKey: ['argus'],
    queryFn: fetchArgus,
    refetchInterval: 120_000,
  })

  const driftStatus = report?.drift_status ?? 'stable'
  const delta =
    report?.drift_recent_score !== undefined && report?.drift_baseline_score !== undefined
      ? report.drift_recent_score - report.drift_baseline_score
      : undefined

  return (
    <div className="space-y-6">
      <div className="page-header">
        <div>
          <h1 className="page-title">Argus</h1>
          <p className="page-subtitle">Quality monitoring and drift analysis</p>
        </div>
      </div>

      {isLoading ? (
        <PageSpinner />
      ) : !report ? (
        <Empty
          icon={<BarChart2 className="w-8 h-8" />}
          title="No Argus report"
          description="The Argus report is generated during the reflect phase after enough sessions."
        />
      ) : (
        <div className="space-y-6">
          {/* Summary stats */}
          <div className="grid grid-cols-2 sm:grid-cols-4 gap-4">
            <StatCard
              label="Review Failures"
              value={String(report.review_failures?.length ?? 0)}
            />
            <StatCard
              label="Eval Failures"
              value={String(report.eval_failures?.length ?? 0)}
            />
            <StatCard
              label="Repeated Work"
              value={String(report.repeated_work?.length ?? 0)}
            />
            <div className="card p-4 rounded-xl border border-slate-100 dark:border-slate-800">
              <p className="text-xs font-medium text-slate-400 uppercase tracking-wider mb-2">Drift</p>
              <div className="flex items-center gap-2">
                {driftIcon(driftStatus)}
                <Badge variant={driftVariant(driftStatus)}>{driftStatus}</Badge>
              </div>
            </div>
          </div>

          {/* Drift scores */}
          {(report.drift_recent_score !== undefined || report.drift_baseline_score !== undefined) && (
            <Card>
              <h2 className="text-sm font-semibold text-slate-700 dark:text-slate-300 mb-3">Drift Analysis</h2>
              <div className="grid grid-cols-1 sm:grid-cols-3 gap-4 text-sm">
                {report.drift_recent_score !== undefined && (
                  <div>
                    <p className="text-xs text-slate-400 uppercase tracking-wider mb-0.5">Recent Avg</p>
                    <p className="font-mono text-slate-700 dark:text-slate-300">
                      {(report.drift_recent_score * 100).toFixed(1)}%
                    </p>
                  </div>
                )}
                {report.drift_baseline_score !== undefined && (
                  <div>
                    <p className="text-xs text-slate-400 uppercase tracking-wider mb-0.5">Baseline Avg</p>
                    <p className="font-mono text-slate-700 dark:text-slate-300">
                      {(report.drift_baseline_score * 100).toFixed(1)}%
                    </p>
                  </div>
                )}
                {delta !== undefined && (
                  <div>
                    <p className="text-xs text-slate-400 uppercase tracking-wider mb-0.5">Delta</p>
                    <p className={`font-mono font-semibold ${
                      delta >= 0
                        ? 'text-emerald-600 dark:text-emerald-400'
                        : 'text-red-600 dark:text-red-400'
                    }`}>
                      {delta >= 0 ? '+' : ''}{(delta * 100).toFixed(1)}%
                    </p>
                  </div>
                )}
              </div>
            </Card>
          )}

          <FailureList title="Review Failures" items={report.review_failures ?? []} />
          <FailureList title="Eval Failures" items={report.eval_failures ?? []} />
          <RepeatedWorkSection report={report} />
          <TokenHotspots report={report} />

          {report.report_text && (
            <Card padding="none">
              <div className="px-5 py-4 border-b border-slate-100 dark:border-slate-800">
                <h2 className="text-sm font-semibold text-slate-700 dark:text-slate-300">Full Report</h2>
              </div>
              <pre className="p-5 text-xs font-mono text-slate-600 dark:text-slate-400 overflow-x-auto whitespace-pre-wrap leading-relaxed">
                {report.report_text}
              </pre>
            </Card>
          )}
        </div>
      )}
    </div>
  )
}
