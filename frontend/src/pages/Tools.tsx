import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Terminal, Wrench, Globe, Cpu, ChevronDown, ChevronRight, CheckCircle, XCircle } from 'lucide-react'
import { fetchTools, type Tool } from '../lib/api'
import { Card } from '../components/ui/Card'
import { Badge } from '../components/ui/Badge'
import { PageSpinner } from '../components/ui/Spinner'
import { Empty } from '../components/ui/Empty'

const kindIcon = (kind: string) => {
  if (kind === 'Shell') return <Terminal className="w-4 h-4" />
  if (kind === 'Http') return <Globe className="w-4 h-4" />
  return <Cpu className="w-4 h-4" />
}

const kindVariant = (kind: string): React.ComponentProps<typeof Badge>['variant'] => {
  if (kind === 'Shell') return 'warning'
  if (kind === 'Http') return 'info'
  return 'default'
}

const levelLabel = (level: number) => {
  if (level === 1) return 'Low'
  if (level === 2) return 'Medium'
  return 'High'
}

const levelVariant = (level: number): React.ComponentProps<typeof Badge>['variant'] => {
  if (level === 1) return 'success'
  if (level === 2) return 'warning'
  return 'danger'
}

function ToolCard({ tool }: { tool: Tool }) {
  const [expanded, setExpanded] = useState(false)

  return (
    <Card padding="none" className="overflow-hidden">
      <div
        className="px-5 py-3.5 flex items-center gap-4 cursor-pointer hover:bg-slate-50 dark:hover:bg-slate-800/50 transition-colors"
        onClick={() => setExpanded((e) => !e)}
      >
        <div className="flex-shrink-0 text-slate-400">
          {kindIcon(tool.kind)}
        </div>
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 flex-wrap">
            <span className="font-mono font-semibold text-sm text-slate-800 dark:text-slate-200">
              {tool.name}
            </span>
            <Badge variant={kindVariant(tool.kind)}>{tool.kind}</Badge>
            <Badge variant={levelVariant(tool.required_level)}>
              Level {tool.required_level} · {levelLabel(tool.required_level)}
            </Badge>
            {tool.requires_approval && (
              <Badge variant="warning">requires approval</Badge>
            )}
          </div>
          <p className="mt-0.5 text-xs text-slate-500 dark:text-slate-400 truncate">
            {tool.description}
          </p>
        </div>
        <div className="flex-shrink-0">
          {expanded ? (
            <ChevronDown className="w-4 h-4 text-slate-400" />
          ) : (
            <ChevronRight className="w-4 h-4 text-slate-400" />
          )}
        </div>
      </div>

      {expanded && (
        <div className="px-5 pb-4 pt-3 border-t border-slate-100 dark:border-slate-800 space-y-3">
          <div className="grid grid-cols-2 sm:grid-cols-3 gap-3 text-sm">
            <div>
              <p className="text-xs text-slate-400 uppercase tracking-wider mb-0.5">Kind</p>
              <p className="font-mono text-slate-700 dark:text-slate-300">{tool.kind}</p>
            </div>
            <div>
              <p className="text-xs text-slate-400 uppercase tracking-wider mb-0.5">Risk Level</p>
              <p className="text-slate-700 dark:text-slate-300">{levelLabel(tool.required_level)}</p>
            </div>
            <div>
              <p className="text-xs text-slate-400 uppercase tracking-wider mb-0.5">Approval</p>
              <div className="flex items-center gap-1.5">
                {tool.requires_approval ? (
                  <><CheckCircle className="w-3.5 h-3.5 text-emerald-500" /><span className="text-emerald-600 dark:text-emerald-400 text-sm">Required</span></>
                ) : (
                  <><XCircle className="w-3.5 h-3.5 text-slate-400" /><span className="text-slate-500 text-sm">Not required</span></>
                )}
              </div>
            </div>
          </div>
          {tool.cooldown_seconds && (
            <div>
              <p className="text-xs text-slate-400 uppercase tracking-wider mb-0.5">Cooldown</p>
              <p className="text-sm text-slate-700 dark:text-slate-300">{tool.cooldown_seconds}s</p>
            </div>
          )}
          {tool.allowed_paths && tool.allowed_paths.length > 0 && (
            <div>
              <p className="text-xs text-slate-400 uppercase tracking-wider mb-1">Allowed Paths</p>
              <div className="flex flex-wrap gap-1.5">
                {tool.allowed_paths.map((p) => (
                  <span key={p} className="badge bg-slate-100 dark:bg-slate-800 text-slate-600 dark:text-slate-300 font-mono">
                    {p}
                  </span>
                ))}
              </div>
            </div>
          )}
        </div>
      )}
    </Card>
  )
}

export function Tools() {
  const [filter, setFilter] = useState<'all' | 'Internal' | 'Shell' | 'Http'>('all')

  const { data: tools = [], isLoading } = useQuery({
    queryKey: ['tools'],
    queryFn: fetchTools,
    refetchInterval: 30_000,
  })

  const filtered = filter === 'all' ? tools : tools.filter((t) => t.kind === filter)

  const counts = {
    Internal: tools.filter((t) => t.kind === 'Internal').length,
    Shell: tools.filter((t) => t.kind === 'Shell').length,
    Http: tools.filter((t) => t.kind === 'Http').length,
  }

  return (
    <div className="space-y-6">
      <div className="page-header">
        <div>
          <h1 className="page-title">Tools</h1>
          <p className="page-subtitle">{tools.length} tools registered</p>
        </div>
      </div>

      <div className="flex gap-1 bg-slate-100 dark:bg-slate-800 rounded-xl p-1 w-fit">
        {(['all', 'Internal', 'Shell', 'Http'] as const).map((f) => (
          <button
            key={f}
            onClick={() => setFilter(f)}
            className={`px-3 py-1.5 text-sm font-medium rounded-lg capitalize transition-all ${
              filter === f
                ? 'bg-white dark:bg-slate-900 shadow-sm text-slate-800 dark:text-slate-200'
                : 'text-slate-500 dark:text-slate-400 hover:text-slate-700 dark:hover:text-slate-300'
            }`}
          >
            {f === 'all' ? `All (${tools.length})` : `${f} (${counts[f]})`}
          </button>
        ))}
      </div>

      {isLoading ? (
        <PageSpinner />
      ) : filtered.length === 0 ? (
        <Empty
          icon={<Wrench className="w-8 h-8" />}
          title="No tools found"
          description="Tool manifests are loaded from the tools/ directory."
        />
      ) : (
        <div className="space-y-2">
          {filtered.map((t) => (
            <ToolCard key={t.name} tool={t} />
          ))}
        </div>
      )}
    </div>
  )
}
