import { useEffect, useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { CheckCircle, ChevronDown, ChevronUp, Search, Shield, XCircle } from 'lucide-react'
import { fetchApprovals, approveRequest, rejectRequest, type Approval } from '../lib/api'
import { Card } from '../components/ui/Card'
import { Badge } from '../components/ui/Badge'
import { PageSpinner } from '../components/ui/Spinner'
import { Empty } from '../components/ui/Empty'
import { formatDate, formatRelative } from '../lib/utils'

const statusVariant = (s: string): React.ComponentProps<typeof Badge>['variant'] => {
  if (s === 'approved') return 'success'
  if (s === 'pending') return 'warning'
  if (s === 'rejected') return 'danger'
  if (s === 'executed') return 'info'
  if (s === 'claiming') return 'default'
  return 'default'
}

function ApprovalCard({ a }: { a: Approval }) {
  const [expanded, setExpanded] = useState(false)
  const qc = useQueryClient()

  const approveMut = useMutation({
    mutationFn: () => approveRequest(a.id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['approvals'] }),
  })
  const rejectMut = useMutation({
    mutationFn: () => rejectRequest(a.id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['approvals'] }),
  })

  const isPending = a.status === 'pending'

  return (
    <Card padding="none" className="overflow-hidden">
      <div
        className="px-5 py-4 flex items-start gap-4 cursor-pointer hover:bg-slate-50 dark:hover:bg-slate-800/50 transition-colors"
        onClick={() => setExpanded((e) => !e)}
      >
        <div className="flex-shrink-0 mt-0.5">
          <Shield className="w-4 h-4 text-slate-400" />
        </div>
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 flex-wrap">
            <span className="text-sm font-semibold text-slate-800 dark:text-slate-200 font-mono">
              {a.tool_name}
            </span>
            <Badge variant={statusVariant(a.status)}>{a.status}</Badge>
            {a.write_paths.length > 0 && (
              <Badge variant="default">{a.write_paths.length} path(s)</Badge>
            )}
          </div>
          <p className="mt-1 text-sm text-slate-600 dark:text-slate-400">{a.summary}</p>
          <div className="mt-1 flex items-center gap-3 text-xs text-slate-400">
            <span>by {a.requested_by}</span>
            <span>{formatRelative(a.created_at)}</span>
          </div>
        </div>
        <div className="flex-shrink-0 flex items-center gap-2">
          {isPending && (
            <>
              <button
                onClick={(e) => {
                  e.stopPropagation()
                  approveMut.mutate()
                }}
                disabled={approveMut.isPending}
                className="btn-success"
              >
                <CheckCircle className="w-3.5 h-3.5" />
                Approve
              </button>
              <button
                onClick={(e) => {
                  e.stopPropagation()
                  rejectMut.mutate()
                }}
                disabled={rejectMut.isPending}
                className="btn-ghost text-red-500 hover:text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20"
              >
                <XCircle className="w-3.5 h-3.5" />
                Reject
              </button>
            </>
          )}
          {expanded ? (
            <ChevronUp className="w-4 h-4 text-slate-400" />
          ) : (
            <ChevronDown className="w-4 h-4 text-slate-400" />
          )}
        </div>
      </div>

      {expanded && (
        <div className="px-5 pb-4 pt-0 border-t border-slate-100 dark:border-slate-800 space-y-3">
          {a.write_paths.length > 0 && (
            <div>
              <p className="text-xs font-medium text-slate-500 dark:text-slate-400 uppercase tracking-wider mb-1">
                Write Paths
              </p>
              <div className="flex flex-wrap gap-1.5">
                {a.write_paths.map((p) => (
                  <span key={p} className="mono badge bg-slate-100 dark:bg-slate-800 text-slate-600 dark:text-slate-300">
                    {p}
                  </span>
                ))}
              </div>
            </div>
          )}
          {a.payload_json && (
            <div>
              <p className="text-xs font-medium text-slate-500 dark:text-slate-400 uppercase tracking-wider mb-1">
                Payload
              </p>
              <pre className="text-xs font-mono bg-slate-50 dark:bg-slate-800 rounded-lg p-3 overflow-x-auto text-slate-700 dark:text-slate-300">
                {(() => {
                  try {
                    return JSON.stringify(JSON.parse(a.payload_json), null, 2)
                  } catch {
                    return a.payload_json
                  }
                })()}
              </pre>
            </div>
          )}
          {a.status_note && (
            <div>
              <p className="text-xs font-medium text-slate-500 dark:text-slate-400 uppercase tracking-wider mb-1">
                Note
              </p>
              <p className="text-sm text-slate-600 dark:text-slate-400">{a.status_note}</p>
            </div>
          )}
          <div className="flex gap-4 text-xs text-slate-400">
            <span>Created: {formatDate(a.created_at)}</span>
            <span>Updated: {formatDate(a.updated_at)}</span>
          </div>
        </div>
      )}
    </Card>
  )
}

export function Approvals() {
  const [filter, setFilter] = useState<'all' | 'pending' | 'approved' | 'rejected' | 'executed' | 'claiming'>(
    'pending',
  )
  const [search, setSearch] = useState('')
  const [debouncedSearch, setDebouncedSearch] = useState('')
  const [toolFilter, setToolFilter] = useState('')

  // Debounce search input
  useEffect(() => {
    const t = setTimeout(() => setDebouncedSearch(search.trim()), 300)
    return () => clearTimeout(t)
  }, [search])

  const statusParam = filter === 'all' ? undefined : filter

  const { data: approvals = [], isLoading } = useQuery({
    queryKey: ['approvals', debouncedSearch, toolFilter, statusParam],
    queryFn: () =>
      fetchApprovals({
        q: debouncedSearch || undefined,
        tool: toolFilter || undefined,
        status: statusParam,
      }),
    refetchInterval: 15_000,
  })

  // Fetch unfiltered list for tool dropdown so tools don't disappear when filters narrow results.
  const { data: allApprovals = [] } = useQuery({
    queryKey: ['approvals', 'all-tools'],
    queryFn: () => fetchApprovals(),
    refetchInterval: 60_000,
  })

  const uniqueTools = Array.from(new Set(allApprovals.map((a) => a.tool_name))).sort()

  const pendingCount = approvals.filter((a) => a.status === 'pending').length

  return (
    <div className="space-y-6">
      <div className="page-header">
        <div>
          <h1 className="page-title">Approvals</h1>
          <p className="page-subtitle">
            {pendingCount > 0 ? `${pendingCount} pending approval(s)` : 'No pending approvals'}
          </p>
        </div>
      </div>

      {/* Search + filters */}
      <div className="flex flex-col sm:flex-row gap-3">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400" />
          <input
            type="text"
            placeholder="Search by tool, summary, or requester…"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="w-full pl-9 pr-3 py-2 text-sm rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-900 text-slate-800 dark:text-slate-200 focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
        </div>
        <select
          value={toolFilter}
          onChange={(e) => setToolFilter(e.target.value)}
          className="px-3 py-2 text-sm rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-900 text-slate-800 dark:text-slate-200 focus:outline-none focus:ring-2 focus:ring-blue-500"
        >
          <option value="">All tools</option>
          {uniqueTools.map((t) => (
            <option key={t} value={t}>
              {t}
            </option>
          ))}
        </select>
      </div>

      {/* Filter tabs */}
      <div className="flex gap-1 bg-slate-100 dark:bg-slate-800 rounded-xl p-1 w-fit">
        {(['all', 'pending', 'approved', 'rejected', 'executed', 'claiming'] as const).map((f) => (
          <button
            key={f}
            onClick={() => setFilter(f)}
            className={`px-3 py-1.5 text-sm font-medium rounded-lg capitalize transition-all ${
              filter === f
                ? 'bg-white dark:bg-slate-900 shadow-sm text-slate-800 dark:text-slate-200'
                : 'text-slate-500 dark:text-slate-400 hover:text-slate-700 dark:hover:text-slate-300'
            }`}
          >
            {f}
            {f === 'pending' && pendingCount > 0 && (
              <span className="ml-1.5 min-w-[1.2rem] h-5 px-1 rounded-full bg-amber-500 text-white text-xs font-bold inline-flex items-center justify-center">
                {pendingCount}
              </span>
            )}
          </button>
        ))}
      </div>

      {isLoading ? (
        <PageSpinner />
      ) : approvals.length === 0 ? (
        <Empty
          icon={<Shield className="w-8 h-8" />}
          title={`No ${filter === 'all' ? '' : filter} approvals`}
          description="Approval requests appear here when tools need authorization."
        />
      ) : (
        <div className="space-y-3">
          {approvals.map((a) => (
            <ApprovalCard key={a.id} a={a} />
          ))}
        </div>
      )}
    </div>
  )
}
