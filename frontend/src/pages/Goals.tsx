import { useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { CheckCircle2, Circle, Plus, Target } from 'lucide-react'
import { fetchGoals, addGoal, type Goal } from '../lib/api'
import { Card } from '../components/ui/Card'
import { PageSpinner } from '../components/ui/Spinner'
import { Empty } from '../components/ui/Empty'
import { Modal } from '../components/ui/Modal'
import { Input } from '../components/ui/Input'

export function Goals() {
  const [showAdd, setShowAdd] = useState(false)
  const [newGoal, setNewGoal] = useState('')
  const qc = useQueryClient()

  const { data: goals = [], isLoading } = useQuery({
    queryKey: ['goals'],
    queryFn: fetchGoals,
    refetchInterval: 60_000,
  })

  const addMut = useMutation({
    mutationFn: () => addGoal(newGoal.trim()),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['goals'] })
      setNewGoal('')
      setShowAdd(false)
    },
  })

  const active = goals.filter((g) => !g.completed)
  const completed = goals.filter((g) => g.completed)

  return (
    <div className="space-y-6">
      <div className="page-header">
        <div>
          <h1 className="page-title">Goals</h1>
          <p className="page-subtitle">
            {active.length} active · {completed.length} completed
          </p>
        </div>
        <button onClick={() => setShowAdd(true)} className="btn-primary">
          <Plus className="w-4 h-4" />
          Add Goal
        </button>
      </div>

      {isLoading ? (
        <PageSpinner />
      ) : goals.length === 0 ? (
        <Empty
          icon={<Target className="w-8 h-8" />}
          title="No goals defined"
          description="Goals are defined in GOALS.md and drive the agent's decision-making."
          action={
            <button onClick={() => setShowAdd(true)} className="btn-primary">
              <Plus className="w-4 h-4" />
              Add Goal
            </button>
          }
        />
      ) : (
        <div className="space-y-6">
          {active.length > 0 && (
            <div>
              <h2 className="section-title">Active ({active.length})</h2>
              <div className="space-y-2">
                {active.map((g, i) => (
                  <GoalRow key={i} goal={g} />
                ))}
              </div>
            </div>
          )}
          {completed.length > 0 && (
            <div>
              <h2 className="section-title">Completed ({completed.length})</h2>
              <div className="space-y-2 opacity-60">
                {completed.map((g, i) => (
                  <GoalRow key={i} goal={g} />
                ))}
              </div>
            </div>
          )}
        </div>
      )}

      <Modal open={showAdd} onClose={() => setShowAdd(false)} title="Add Goal">
        <div className="space-y-4">
          <Input
            label="Goal description"
            value={newGoal}
            onChange={(e) => setNewGoal(e.target.value)}
            placeholder="e.g. Improve error handling across the codebase"
            onKeyDown={(e) => {
              if (e.key === 'Enter') addMut.mutate()
            }}
          />
          {addMut.isError && (
            <p className="text-sm text-red-500">
              {addMut.error instanceof Error ? addMut.error.message : 'Failed to add goal'}
            </p>
          )}
          <div className="flex gap-3 justify-end">
            <button onClick={() => setShowAdd(false)} className="btn-secondary">
              Cancel
            </button>
            <button
              onClick={() => addMut.mutate()}
              disabled={!newGoal.trim() || addMut.isPending}
              className="btn-primary"
            >
              {addMut.isPending ? 'Adding…' : 'Add Goal'}
            </button>
          </div>
        </div>
      </Modal>
    </div>
  )
}

function GoalRow({ goal }: { goal: Goal }) {
  return (
    <Card className="flex items-center gap-3">
      <div className="flex-shrink-0">
        {goal.completed ? (
          <CheckCircle2 className="w-5 h-5 text-emerald-500" />
        ) : (
          <Circle className="w-5 h-5 text-slate-300 dark:text-slate-600" />
        )}
      </div>
      <div className="flex-1 min-w-0">
        <p className={`text-sm font-medium ${goal.completed ? 'line-through text-slate-400' : 'text-slate-800 dark:text-slate-200'}`}>
          {goal.title}
        </p>
        {goal.raw_id && (
          <p className="text-xs mono text-slate-400 mt-0.5">{goal.raw_id}</p>
        )}
      </div>
    </Card>
  )
}
