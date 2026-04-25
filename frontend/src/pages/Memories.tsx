import { useEffect, useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Brain, RefreshCcw, Search, Star, Trash2, X } from 'lucide-react'
import {
  fetchHotMemories,
  fetchColdMemories,
  searchMemories,
  reinforceMemory,
  forgetMemory,
  consolidateMemories,
  type Memory,
} from '../lib/api'
import { Card } from '../components/ui/Card'
import { Badge } from '../components/ui/Badge'
import { PageSpinner } from '../components/ui/Spinner'
import { Empty } from '../components/ui/Empty'

function MemoryCard({
  m,
  onReinforce,
  onForget,
}: {
  m: Memory
  onReinforce: () => void
  onForget: () => void
}) {
  return (
    <Card className="group">
      <div className="flex items-start gap-3">
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 flex-wrap mb-1.5">
            <Badge variant={m.tier === 'hot' ? 'warning' : 'info'}>{m.tier}</Badge>
            <Badge variant="default">{m.memory_type}</Badge>
            <span className="text-xs font-mono text-slate-400">
              score: {m.score.toFixed(3)}
            </span>
          </div>
          <p className="text-sm text-slate-700 dark:text-slate-300 leading-relaxed">
            {m.content}
          </p>
          {m.summary && m.summary !== m.content && (
            <p className="mt-1 text-xs text-slate-500 dark:text-slate-400 italic">
              {m.summary}
            </p>
          )}
          {m.tags.length > 0 && (
            <div className="mt-2 flex flex-wrap gap-1">
              {m.tags.map((t) => (
                <span
                  key={t}
                  className="badge bg-slate-100 dark:bg-slate-800 text-slate-500 dark:text-slate-400"
                >
                  #{t}
                </span>
              ))}
            </div>
          )}
        </div>
        <div className="flex-shrink-0 flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
          <button
            onClick={onReinforce}
            className="btn-ghost text-amber-500 hover:bg-amber-50 dark:hover:bg-amber-900/20"
            title="Reinforce"
          >
            <Star className="w-3.5 h-3.5" />
          </button>
          <button
            onClick={onForget}
            className="btn-ghost text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20"
            title="Forget"
          >
            <Trash2 className="w-3.5 h-3.5" />
          </button>
        </div>
      </div>
    </Card>
  )
}

export function Memories() {
  const [tab, setTab] = useState<'hot' | 'cold'>('hot')
  const [search, setSearch] = useState('')
  const [debouncedSearch, setDebouncedSearch] = useState('')
  const qc = useQueryClient()

  useEffect(() => {
    const t = setTimeout(() => setDebouncedSearch(search.trim()), 300)
    return () => clearTimeout(t)
  }, [search])

  const isSearching = debouncedSearch.length > 0

  const { data: hot = [], isLoading: hotLoading } = useQuery({
    queryKey: ['memories', 'hot'],
    queryFn: fetchHotMemories,
    refetchInterval: 60_000,
    enabled: !isSearching,
  })
  const { data: cold = [], isLoading: coldLoading } = useQuery({
    queryKey: ['memories', 'cold'],
    queryFn: fetchColdMemories,
    refetchInterval: 60_000,
    enabled: tab === 'cold' && !isSearching,
  })
  const { data: searchResults = [], isLoading: searchLoading } = useQuery({
    queryKey: ['memories', 'search', debouncedSearch],
    queryFn: () => searchMemories(debouncedSearch),
    enabled: isSearching,
  })

  const consolidateMut = useMutation({
    mutationFn: consolidateMemories,
    onSuccess: (res) => {
      alert(`Consolidated ${res.consolidated}, pruned ${res.pruned}`)
      qc.invalidateQueries({ queryKey: ['memories'] })
    },
  })

  const reinforceMut = useMutation({
    mutationFn: reinforceMemory,
    onSuccess: () => qc.invalidateQueries({ queryKey: ['memories'] }),
  })

  const forgetMut = useMutation({
    mutationFn: forgetMemory,
    onSuccess: () => qc.invalidateQueries({ queryKey: ['memories'] }),
  })

  const memories = isSearching ? searchResults : tab === 'hot' ? hot : cold
  const loading = isSearching
    ? searchLoading
    : tab === 'hot'
      ? hotLoading
      : coldLoading

  return (
    <div className="space-y-6">
      <div className="page-header">
        <div>
          <h1 className="page-title">Memories</h1>
          <p className="page-subtitle">
            {hot.length} hot · {cold.length} cold
          </p>
        </div>
        <button
          onClick={() => consolidateMut.mutate()}
          disabled={consolidateMut.isPending}
          className="btn-secondary"
        >
          <RefreshCcw className="w-4 h-4" />
          Consolidate
        </button>
      </div>

      {/* Search */}
      <div className="relative">
        <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400" />
        <input
          type="text"
          placeholder="Search memories…"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="w-full pl-9 pr-9 py-2 text-sm rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-900 text-slate-800 dark:text-slate-200 focus:outline-none focus:ring-2 focus:ring-blue-500"
        />
        {search && (
          <button
            onClick={() => setSearch('')}
            className="absolute right-3 top-1/2 -translate-y-1/2 text-slate-400 hover:text-slate-600"
          >
            <X className="w-4 h-4" />
          </button>
        )}
      </div>

      {/* Tabs — hide when searching, since search covers both tiers */}
      {!isSearching && (
        <div className="flex gap-1 bg-slate-100 dark:bg-slate-800 rounded-xl p-1 w-fit">
          {(['hot', 'cold'] as const).map((t) => (
            <button
              key={t}
              onClick={() => setTab(t)}
              className={`px-4 py-1.5 text-sm font-medium rounded-lg capitalize transition-all ${
                tab === t
                  ? 'bg-white dark:bg-slate-900 shadow-sm text-slate-800 dark:text-slate-200'
                  : 'text-slate-500 dark:text-slate-400'
              }`}
            >
              {t === 'hot' ? `🔥 Hot (${hot.length})` : `❄️ Cold (${cold.length})`}
            </button>
          ))}
        </div>
      )}

      {isSearching && (
        <p className="text-xs text-slate-500">
          {searchResults.length} result(s) for "{debouncedSearch}"
        </p>
      )}

      {loading ? (
        <PageSpinner />
      ) : memories.length === 0 ? (
        <Empty
          icon={<Brain className="w-8 h-8" />}
          title={isSearching ? 'No matches' : `No ${tab} memories`}
          description={
            isSearching
              ? 'Try a different search term.'
              : tab === 'hot'
                ? 'Hot memories are created during sessions.'
                : 'Cold memories are promoted from hot memory over time.'
          }
        />
      ) : (
        <div className="space-y-3">
          {memories.map((m) => (
            <MemoryCard
              key={m.id}
              m={m}
              onReinforce={() => reinforceMut.mutate(m.id)}
              onForget={() => forgetMut.mutate(m.id)}
            />
          ))}
        </div>
      )}
    </div>
  )
}
