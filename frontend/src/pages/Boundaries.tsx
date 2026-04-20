import { useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Plus, ShieldOff, Trash2 } from 'lucide-react'
import {
  fetchBoundaries, addBoundary, confirmBoundary, removeBoundary, type Boundary,
} from '../lib/api'
import { Card } from '../components/ui/Card'
import { Badge } from '../components/ui/Badge'
import { PageSpinner } from '../components/ui/Spinner'
import { Empty } from '../components/ui/Empty'
import { Modal } from '../components/ui/Modal'
import { Input, Textarea } from '../components/ui/Input'
import { formatRelative } from '../lib/utils'

const categoryVariant = (c: string): React.ComponentProps<typeof Badge>['variant'] => {
  if (c === 'hard') return 'danger'
  if (c === 'soft') return 'warning'
  return 'default'
}

function BoundaryCard({ b, onConfirm, onRemove }: {
  b: Boundary
  onConfirm: () => void
  onRemove: () => void
}) {
  return (
    <Card className="group">
      <div className="flex items-start gap-3">
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 flex-wrap mb-1.5">
            <Badge variant={categoryVariant(b.category)}>{b.category}</Badge>
            {!b.confirmed && <Badge variant="warning">unconfirmed</Badge>}
            <span className="text-xs text-slate-400">{formatRelative(b.created_at)}</span>
          </div>
          <p className="text-sm font-medium text-slate-800 dark:text-slate-200">{b.description}</p>
          {b.rationale && (
            <p className="mt-1 text-sm text-slate-500 dark:text-slate-400 italic">{b.rationale}</p>
          )}
          {b.source && (
            <p className="mt-1.5 text-xs text-slate-400 font-mono">source: {b.source}</p>
          )}
        </div>
        <div className="flex-shrink-0 flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
          {!b.confirmed && (
            <button
              onClick={onConfirm}
              className="btn-ghost text-emerald-500 hover:bg-emerald-50 dark:hover:bg-emerald-900/20 text-xs"
            >
              Confirm
            </button>
          )}
          <button
            onClick={onRemove}
            className="btn-ghost text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20"
            title="Remove"
          >
            <Trash2 className="w-3.5 h-3.5" />
          </button>
        </div>
      </div>
    </Card>
  )
}

export function Boundaries() {
  const [showAdd, setShowAdd] = useState(false)
  const [form, setForm] = useState({ description: '', category: 'hard', rationale: '' })
  const qc = useQueryClient()

  const { data: boundaries = [], isLoading } = useQuery({
    queryKey: ['boundaries'],
    queryFn: fetchBoundaries,
    refetchInterval: 60_000,
  })

  const addMut = useMutation({
    mutationFn: () => addBoundary(form.description, form.category, form.rationale || undefined),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['boundaries'] })
      setForm({ description: '', category: 'hard', rationale: '' })
      setShowAdd(false)
    },
  })

  const confirmMut = useMutation({
    mutationFn: (id: number) => confirmBoundary(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['boundaries'] }),
  })

  const removeMut = useMutation({
    mutationFn: (id: number) => removeBoundary(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['boundaries'] }),
  })

  const hard = boundaries.filter((b) => b.category === 'hard')
  const soft = boundaries.filter((b) => b.category === 'soft')
  const other = boundaries.filter((b) => b.category !== 'hard' && b.category !== 'soft')

  return (
    <div className="space-y-6">
      <div className="page-header">
        <div>
          <h1 className="page-title">Boundaries</h1>
          <p className="page-subtitle">{boundaries.length} boundaries defined</p>
        </div>
        <button onClick={() => setShowAdd(true)} className="btn-primary">
          <Plus className="w-4 h-4" />
          Add Boundary
        </button>
      </div>

      {isLoading ? (
        <PageSpinner />
      ) : boundaries.length === 0 ? (
        <Empty
          icon={<ShieldOff className="w-8 h-8" />}
          title="No boundaries defined"
          description="Boundaries constrain agent behavior. Hard limits are never crossed; soft limits require justification."
          action={
            <button onClick={() => setShowAdd(true)} className="btn-primary">
              <Plus className="w-4 h-4" />
              Add Boundary
            </button>
          }
        />
      ) : (
        <div className="space-y-6">
          {hard.length > 0 && (
            <div>
              <h2 className="section-title">Hard Limits ({hard.length})</h2>
              <div className="space-y-3">
                {hard.map((b) => (
                  <BoundaryCard
                    key={b.id}
                    b={b}
                    onConfirm={() => confirmMut.mutate(b.id)}
                    onRemove={() => removeMut.mutate(b.id)}
                  />
                ))}
              </div>
            </div>
          )}
          {soft.length > 0 && (
            <div>
              <h2 className="section-title">Soft Limits ({soft.length})</h2>
              <div className="space-y-3">
                {soft.map((b) => (
                  <BoundaryCard
                    key={b.id}
                    b={b}
                    onConfirm={() => confirmMut.mutate(b.id)}
                    onRemove={() => removeMut.mutate(b.id)}
                  />
                ))}
              </div>
            </div>
          )}
          {other.length > 0 && (
            <div>
              <h2 className="section-title">Other ({other.length})</h2>
              <div className="space-y-3">
                {other.map((b) => (
                  <BoundaryCard
                    key={b.id}
                    b={b}
                    onConfirm={() => confirmMut.mutate(b.id)}
                    onRemove={() => removeMut.mutate(b.id)}
                  />
                ))}
              </div>
            </div>
          )}
        </div>
      )}

      <Modal open={showAdd} onClose={() => setShowAdd(false)} title="Add Boundary">
        <div className="space-y-4">
          <Input
            label="Description"
            value={form.description}
            onChange={(e) => setForm((f) => ({ ...f, description: e.target.value }))}
            placeholder="e.g. Never delete files without explicit confirmation"
          />
          <div className="space-y-1">
            <label className="text-sm font-medium text-slate-600 dark:text-slate-300">Category</label>
            <select
              value={form.category}
              onChange={(e) => setForm((f) => ({ ...f, category: e.target.value }))}
              className="w-full px-3 py-2 text-sm rounded-lg border border-slate-200 dark:border-slate-700
                         bg-white dark:bg-slate-800 text-slate-700 dark:text-slate-200
                         focus:outline-none focus:ring-2 focus:ring-brand-500/20 focus:border-brand-500"
            >
              <option value="hard">Hard limit</option>
              <option value="soft">Soft limit</option>
              <option value="preference">Preference</option>
            </select>
          </div>
          <Textarea
            label="Rationale (optional)"
            value={form.rationale}
            onChange={(e) => setForm((f) => ({ ...f, rationale: e.target.value }))}
            placeholder="Why does this boundary exist?"
            rows={3}
          />
          {addMut.isError && (
            <p className="text-sm text-red-500">
              {addMut.error instanceof Error ? addMut.error.message : 'Failed to add boundary'}
            </p>
          )}
          <div className="flex gap-3 justify-end">
            <button onClick={() => setShowAdd(false)} className="btn-secondary">Cancel</button>
            <button
              onClick={() => addMut.mutate()}
              disabled={!form.description.trim() || addMut.isPending}
              className="btn-primary"
            >
              {addMut.isPending ? 'Adding…' : 'Add Boundary'}
            </button>
          </div>
        </div>
      </Modal>
    </div>
  )
}
