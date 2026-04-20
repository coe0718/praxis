import { useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { BookOpen, Plus, Zap } from 'lucide-react'
import { fetchLearningOpportunities, triggerLearningRun, addLearningNote, type LearningOpportunity } from '../lib/api'
import { Card } from '../components/ui/Card'
import { Badge } from '../components/ui/Badge'
import { PageSpinner } from '../components/ui/Spinner'
import { Empty } from '../components/ui/Empty'
import { Modal } from '../components/ui/Modal'
import { Textarea } from '../components/ui/Input'
import { formatRelative } from '../lib/utils'

const priorityVariant = (p: string): React.ComponentProps<typeof Badge>['variant'] => {
  if (p === 'high') return 'danger'
  if (p === 'medium') return 'warning'
  return 'default'
}

function OpportunityCard({ op }: { op: LearningOpportunity }) {
  return (
    <Card>
      <div className="flex items-start gap-3">
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 flex-wrap mb-1.5">
            <Badge variant={priorityVariant(op.priority)}>{op.priority}</Badge>
            <Badge variant="default">{op.category}</Badge>
            <span className="text-xs text-slate-400">{formatRelative(op.created_at)}</span>
          </div>
          <p className="text-sm font-medium text-slate-800 dark:text-slate-200">{op.title}</p>
          <p className="mt-1 text-sm text-slate-600 dark:text-slate-400 leading-relaxed">
            {op.description}
          </p>
          {op.source && (
            <p className="mt-1.5 text-xs text-slate-400 font-mono">source: {op.source}</p>
          )}
        </div>
      </div>
    </Card>
  )
}

export function Learning() {
  const [showNote, setShowNote] = useState(false)
  const [noteText, setNoteText] = useState('')
  const qc = useQueryClient()

  const { data: opportunities = [], isLoading } = useQuery({
    queryKey: ['learning', 'opportunities'],
    queryFn: fetchLearningOpportunities,
    refetchInterval: 60_000,
  })

  const runMut = useMutation({
    mutationFn: triggerLearningRun,
    onSuccess: (res) => {
      alert(`Learning run complete. Found ${res.opportunities_found ?? 0} opportunities.`)
      qc.invalidateQueries({ queryKey: ['learning'] })
    },
  })

  const noteMut = useMutation({
    mutationFn: () => addLearningNote(noteText.trim()),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['learning'] })
      setNoteText('')
      setShowNote(false)
    },
  })

  const high = opportunities.filter((o) => o.priority === 'high')
  const medium = opportunities.filter((o) => o.priority === 'medium')
  const low = opportunities.filter((o) => o.priority !== 'high' && o.priority !== 'medium')

  return (
    <div className="space-y-6">
      <div className="page-header">
        <div>
          <h1 className="page-title">Learning</h1>
          <p className="page-subtitle">{opportunities.length} opportunities identified</p>
        </div>
        <div className="flex gap-2">
          <button onClick={() => setShowNote(true)} className="btn-secondary">
            <Plus className="w-4 h-4" />
            Add Note
          </button>
          <button
            onClick={() => runMut.mutate()}
            disabled={runMut.isPending}
            className="btn-primary"
          >
            <Zap className="w-4 h-4" />
            {runMut.isPending ? 'Running…' : 'Run Learning'}
          </button>
        </div>
      </div>

      {isLoading ? (
        <PageSpinner />
      ) : opportunities.length === 0 ? (
        <Empty
          icon={<BookOpen className="w-8 h-8" />}
          title="No learning opportunities"
          description="Run a learning cycle to mine the Argus report for insights."
          action={
            <button onClick={() => runMut.mutate()} disabled={runMut.isPending} className="btn-primary">
              <Zap className="w-4 h-4" />
              Run Learning
            </button>
          }
        />
      ) : (
        <div className="space-y-6">
          {high.length > 0 && (
            <div>
              <h2 className="section-title">High Priority ({high.length})</h2>
              <div className="space-y-3">
                {high.map((op, i) => <OpportunityCard key={i} op={op} />)}
              </div>
            </div>
          )}
          {medium.length > 0 && (
            <div>
              <h2 className="section-title">Medium Priority ({medium.length})</h2>
              <div className="space-y-3">
                {medium.map((op, i) => <OpportunityCard key={i} op={op} />)}
              </div>
            </div>
          )}
          {low.length > 0 && (
            <div>
              <h2 className="section-title">Low Priority ({low.length})</h2>
              <div className="space-y-3">
                {low.map((op, i) => <OpportunityCard key={i} op={op} />)}
              </div>
            </div>
          )}
        </div>
      )}

      <Modal open={showNote} onClose={() => setShowNote(false)} title="Add Learning Note">
        <div className="space-y-4">
          <Textarea
            label="Note"
            value={noteText}
            onChange={(e) => setNoteText(e.target.value)}
            placeholder="Describe a pattern, mistake, or insight to record…"
            rows={5}
          />
          {noteMut.isError && (
            <p className="text-sm text-red-500">
              {noteMut.error instanceof Error ? noteMut.error.message : 'Failed to add note'}
            </p>
          )}
          <div className="flex gap-3 justify-end">
            <button onClick={() => setShowNote(false)} className="btn-secondary">Cancel</button>
            <button
              onClick={() => noteMut.mutate()}
              disabled={!noteText.trim() || noteMut.isPending}
              className="btn-primary"
            >
              {noteMut.isPending ? 'Saving…' : 'Save Note'}
            </button>
          </div>
        </div>
      </Modal>
    </div>
  )
}
