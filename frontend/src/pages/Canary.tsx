import { useState } from 'react'
import { useMutation } from '@tanstack/react-query'
import { Bird, Play, CheckCircle, XCircle, AlertCircle } from 'lucide-react'
import { runCanary, type CanaryResult } from '../lib/api'
import { Card } from '../components/ui/Card'
import { Badge } from '../components/ui/Badge'
import { Empty } from '../components/ui/Empty'

function ResultRow({ r }: { r: CanaryResult }) {
  const icon = r.passed
    ? <CheckCircle className="w-4 h-4 text-emerald-500 flex-shrink-0" />
    : <XCircle className="w-4 h-4 text-red-500 flex-shrink-0" />

  return (
    <div className="flex items-start gap-3 py-3 border-b border-slate-100 dark:border-slate-800 last:border-0">
      {icon}
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2 flex-wrap">
          <span className="font-mono text-sm font-medium text-slate-800 dark:text-slate-200">
            {r.test_name}
          </span>
          <Badge variant={r.passed ? 'success' : 'danger'}>{r.passed ? 'pass' : 'fail'}</Badge>
          {r.latency_ms !== undefined && (
            <span className="text-xs text-slate-400">{r.latency_ms}ms</span>
          )}
        </div>
        {r.message && (
          <p className="mt-0.5 text-sm text-slate-500 dark:text-slate-400">{r.message}</p>
        )}
      </div>
    </div>
  )
}

export function Canary() {
  const [results, setResults] = useState<CanaryResult[] | null>(null)
  const [provider, setProvider] = useState('')

  const runMut = useMutation({
    mutationFn: () => runCanary(provider.trim() || undefined),
    onSuccess: (data) => {
      setResults(data)
    },
  })

  const passed = results?.filter((r) => r.passed).length ?? 0
  const total = results?.length ?? 0
  const allPassed = results && passed === total

  return (
    <div className="space-y-6">
      <div className="page-header">
        <div>
          <h1 className="page-title">Canary</h1>
          <p className="page-subtitle">Smoke-test the agent's LLM providers</p>
        </div>
        <button
          onClick={() => runMut.mutate()}
          disabled={runMut.isPending}
          className="btn-primary"
        >
          <Play className="w-4 h-4" />
          {runMut.isPending ? 'Running…' : 'Run Canary'}
        </button>
      </div>

      <Card>
        <div className="flex items-center gap-3">
          <label className="text-sm font-medium text-slate-600 dark:text-slate-300 flex-shrink-0">
            Provider (optional)
          </label>
          <input
            type="text"
            value={provider}
            onChange={(e) => setProvider(e.target.value)}
            placeholder="e.g. openai, anthropic — leave blank to test all"
            className="flex-1 px-3 py-1.5 text-sm rounded-lg border border-slate-200 dark:border-slate-700
                       bg-white dark:bg-slate-800 text-slate-800 dark:text-slate-200
                       focus:outline-none focus:ring-2 focus:ring-brand-500/20 focus:border-brand-500"
          />
        </div>
      </Card>

      {runMut.isPending && (
        <Card>
          <div className="flex items-center gap-3 text-slate-500">
            <div className="w-5 h-5 border-2 border-brand-500 border-t-transparent rounded-full animate-spin" />
            <span className="text-sm">Running canary tests…</span>
          </div>
        </Card>
      )}

      {runMut.isError && (
        <Card>
          <div className="flex items-center gap-3 text-red-500">
            <AlertCircle className="w-5 h-5 flex-shrink-0" />
            <p className="text-sm">
              {runMut.error instanceof Error ? runMut.error.message : 'Canary run failed'}
            </p>
          </div>
        </Card>
      )}

      {results && (
        <div className="space-y-4">
          <div className="flex items-center gap-3">
            {allPassed ? (
              <CheckCircle className="w-5 h-5 text-emerald-500" />
            ) : (
              <XCircle className="w-5 h-5 text-red-500" />
            )}
            <span className={`font-semibold ${allPassed ? 'text-emerald-600 dark:text-emerald-400' : 'text-red-600 dark:text-red-400'}`}>
              {passed}/{total} tests passed
            </span>
          </div>

          <Card padding="none">
            <div className="px-5 divide-y divide-slate-100 dark:divide-slate-800">
              {results.length === 0 ? (
                <Empty
                  icon={<Bird className="w-8 h-8" />}
                  title="No tests returned"
                  description="The canary run completed but produced no results."
                />
              ) : (
                results.map((r, i) => <ResultRow key={i} r={r} />)
              )}
            </div>
          </Card>
        </div>
      )}

      {!results && !runMut.isPending && !runMut.isError && (
        <Empty
          icon={<Bird className="w-8 h-8" />}
          title="No results yet"
          description="Click 'Run Canary' to smoke-test your LLM providers."
          action={
            <button onClick={() => runMut.mutate()} className="btn-primary">
              <Play className="w-4 h-4" />
              Run Canary
            </button>
          }
        />
      )}
    </div>
  )
}
