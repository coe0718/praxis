import { useQuery } from '@tanstack/react-query'
import { Link2, ArrowRight } from 'lucide-react'
import { fetchDelegation, type DelegationLink } from '../lib/api'
import { Card } from '../components/ui/Card'
import { Badge } from '../components/ui/Badge'
import { PageSpinner } from '../components/ui/Spinner'
import { Empty } from '../components/ui/Empty'
import { formatRelative } from '../lib/utils'

const statusVariant = (s: string): React.ComponentProps<typeof Badge>['variant'] => {
  if (s === 'active') return 'success'
  if (s === 'pending') return 'warning'
  if (s === 'failed') return 'danger'
  return 'default'
}

function DelegationCard({ link }: { link: DelegationLink }) {
  return (
    <Card>
      <div className="flex items-start gap-4">
        <div className="flex-shrink-0 mt-0.5">
          <Link2 className="w-4 h-4 text-slate-400" />
        </div>
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 flex-wrap mb-1.5">
            <Badge variant={statusVariant(link.status)}>{link.status}</Badge>
            <span className="text-xs text-slate-400">{formatRelative(link.created_at)}</span>
          </div>
          <div className="flex items-center gap-2 text-sm font-mono">
            <span className="font-semibold text-slate-800 dark:text-slate-200">{link.from_agent}</span>
            <ArrowRight className="w-3.5 h-3.5 text-slate-400 flex-shrink-0" />
            <span className="font-semibold text-brand-600 dark:text-brand-400">{link.to_agent}</span>
          </div>
          {link.task_summary && (
            <p className="mt-1.5 text-sm text-slate-600 dark:text-slate-400 leading-relaxed">
              {link.task_summary}
            </p>
          )}
          {link.result_summary && (
            <div className="mt-2 px-3 py-2 bg-emerald-50 dark:bg-emerald-900/20 rounded-lg text-xs text-emerald-700 dark:text-emerald-300">
              {link.result_summary}
            </div>
          )}
          {link.error && (
            <div className="mt-2 px-3 py-2 bg-red-50 dark:bg-red-900/20 rounded-lg text-xs text-red-700 dark:text-red-300">
              {link.error}
            </div>
          )}
        </div>
      </div>
    </Card>
  )
}

export function Delegation() {
  const { data: links = [], isLoading } = useQuery({
    queryKey: ['delegation'],
    queryFn: fetchDelegation,
    refetchInterval: 30_000,
  })

  const active = links.filter((l) => l.status === 'active')
  const completed = links.filter((l) => l.status !== 'active' && l.status !== 'pending')
  const pending = links.filter((l) => l.status === 'pending')

  return (
    <div className="space-y-6">
      <div className="page-header">
        <div>
          <h1 className="page-title">Delegation</h1>
          <p className="page-subtitle">{links.length} delegation links</p>
        </div>
      </div>

      <div className="px-4 py-3 bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 rounded-xl text-sm text-amber-700 dark:text-amber-300">
        Delegation is implemented at the store layer. The Act phase does not yet send work over links — this is a stub view of the data store.
      </div>

      {isLoading ? (
        <PageSpinner />
      ) : links.length === 0 ? (
        <Empty
          icon={<Link2 className="w-8 h-8" />}
          title="No delegation links"
          description="Delegation links appear when the agent hands off tasks to sub-agents."
        />
      ) : (
        <div className="space-y-6">
          {active.length > 0 && (
            <div>
              <h2 className="section-title">Active ({active.length})</h2>
              <div className="space-y-3">
                {active.map((l) => <DelegationCard key={l.id} link={l} />)}
              </div>
            </div>
          )}
          {pending.length > 0 && (
            <div>
              <h2 className="section-title">Pending ({pending.length})</h2>
              <div className="space-y-3">
                {pending.map((l) => <DelegationCard key={l.id} link={l} />)}
              </div>
            </div>
          )}
          {completed.length > 0 && (
            <div>
              <h2 className="section-title">Completed ({completed.length})</h2>
              <div className="space-y-3 opacity-70">
                {completed.map((l) => <DelegationCard key={l.id} link={l} />)}
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  )
}
