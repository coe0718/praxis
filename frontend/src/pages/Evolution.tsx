import { useQuery } from '@tanstack/react-query'
import { Dna, FileText } from 'lucide-react'
import { fetchEvolution, type EvolutionProposal } from '../lib/api'
import { Card } from '../components/ui/Card'
import { Badge } from '../components/ui/Badge'
import { PageSpinner } from '../components/ui/Spinner'
import { Empty } from '../components/ui/Empty'
import { formatRelative } from '../lib/utils'

const statusVariant = (s: string): React.ComponentProps<typeof Badge>['variant'] => {
  if (s === 'approved') return 'success'
  if (s === 'pending') return 'warning'
  if (s === 'rejected') return 'danger'
  if (s === 'applied') return 'info'
  return 'default'
}

const typeVariant = (t: string): React.ComponentProps<typeof Badge>['variant'] => {
  if (t === 'Identity') return 'info'
  if (t === 'Config') return 'warning'
  if (t === 'Capability') return 'success'
  return 'default'
}

function ProposalCard({ p }: { p: EvolutionProposal }) {
  return (
    <Card>
      <div className="flex items-start gap-3">
        <div className="flex-shrink-0 mt-0.5">
          <Dna className="w-4 h-4 text-brand-400" />
        </div>
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 flex-wrap mb-1.5">
            <Badge variant={typeVariant(p.proposal_type)}>{p.proposal_type}</Badge>
            <Badge variant={statusVariant(p.status)}>{p.status}</Badge>
            <span className="text-xs text-slate-400">{formatRelative(p.created_at)}</span>
          </div>
          <p className="text-sm font-semibold text-slate-800 dark:text-slate-200">{p.title}</p>
          <p className="mt-1.5 text-sm text-slate-600 dark:text-slate-400 leading-relaxed">
            {p.rationale}
          </p>
          {p.proposed_change && (
            <details className="mt-3">
              <summary className="text-xs text-slate-400 cursor-pointer hover:text-slate-600 dark:hover:text-slate-300 select-none">
                Proposed change
              </summary>
              <pre className="mt-2 text-xs font-mono bg-slate-50 dark:bg-slate-800 rounded-lg p-3 overflow-x-auto text-slate-700 dark:text-slate-300 whitespace-pre-wrap leading-relaxed">
                {p.proposed_change}
              </pre>
            </details>
          )}
          {p.session_id && (
            <p className="mt-2 text-xs text-slate-400 font-mono">session #{p.session_id}</p>
          )}
        </div>
      </div>
    </Card>
  )
}

export function Evolution() {
  const { data: proposals = [], isLoading } = useQuery({
    queryKey: ['evolution'],
    queryFn: fetchEvolution,
    refetchInterval: 60_000,
  })

  const pending = proposals.filter((p) => p.status === 'pending')
  const applied = proposals.filter((p) => p.status === 'applied' || p.status === 'approved')
  const rejected = proposals.filter((p) => p.status === 'rejected')

  return (
    <div className="space-y-6">
      <div className="page-header">
        <div>
          <h1 className="page-title">Evolution</h1>
          <p className="page-subtitle">
            {pending.length} pending · {applied.length} applied · {rejected.length} rejected
          </p>
        </div>
      </div>

      {isLoading ? (
        <PageSpinner />
      ) : proposals.length === 0 ? (
        <Empty
          icon={<FileText className="w-8 h-8" />}
          title="No evolution proposals"
          description="The agent generates proposals after reflecting on sessions with low scores or failed reviews."
        />
      ) : (
        <div className="space-y-6">
          {pending.length > 0 && (
            <div>
              <h2 className="section-title">Pending ({pending.length})</h2>
              <div className="space-y-3">
                {pending.map((p, i) => <ProposalCard key={i} p={p} />)}
              </div>
            </div>
          )}
          {applied.length > 0 && (
            <div>
              <h2 className="section-title">Applied ({applied.length})</h2>
              <div className="space-y-3 opacity-75">
                {applied.map((p, i) => <ProposalCard key={i} p={p} />)}
              </div>
            </div>
          )}
          {rejected.length > 0 && (
            <div>
              <h2 className="section-title">Rejected ({rejected.length})</h2>
              <div className="space-y-3 opacity-60">
                {rejected.map((p, i) => <ProposalCard key={i} p={p} />)}
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  )
}
