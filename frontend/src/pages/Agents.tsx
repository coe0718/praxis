import { useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Bot, Plus } from 'lucide-react'
import { fetchAgents, addAgent, type AgentEntry } from '../lib/api'
import { Card } from '../components/ui/Card'
import { Badge } from '../components/ui/Badge'
import { PageSpinner } from '../components/ui/Spinner'
import { Empty } from '../components/ui/Empty'
import { Modal } from '../components/ui/Modal'
import { Input, Textarea } from '../components/ui/Input'
import { formatRelative } from '../lib/utils'

const roleVariant = (r: string): React.ComponentProps<typeof Badge>['variant'] => {
  if (r === 'orchestrator') return 'info'
  if (r === 'worker') return 'default'
  if (r === 'reviewer') return 'warning'
  return 'default'
}

function AgentCard({ agent }: { agent: AgentEntry }) {
  return (
    <Card>
      <div className="flex items-start gap-3">
        <div className="flex-shrink-0 mt-0.5">
          <Bot className="w-5 h-5 text-brand-400" />
        </div>
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 flex-wrap mb-1">
            <span className="font-semibold text-sm text-slate-800 dark:text-slate-200">{agent.name}</span>
            {agent.role && <Badge variant={roleVariant(agent.role)}>{agent.role}</Badge>}
            <span className="text-xs text-slate-400">{formatRelative(agent.created_at)}</span>
          </div>
          {agent.description && (
            <p className="text-sm text-slate-600 dark:text-slate-400 leading-relaxed">
              {agent.description}
            </p>
          )}
          {agent.endpoint && (
            <p className="mt-1.5 text-xs font-mono text-slate-400 truncate">{agent.endpoint}</p>
          )}
          {agent.capabilities && agent.capabilities.length > 0 && (
            <div className="mt-2 flex flex-wrap gap-1">
              {agent.capabilities.map((c) => (
                <span key={c} className="badge bg-slate-100 dark:bg-slate-800 text-slate-500 dark:text-slate-400">
                  {c}
                </span>
              ))}
            </div>
          )}
        </div>
      </div>
    </Card>
  )
}

export function Agents() {
  const [showAdd, setShowAdd] = useState(false)
  const [form, setForm] = useState({ name: '', role: 'worker', description: '', endpoint: '' })
  const qc = useQueryClient()

  const { data: agents = [], isLoading } = useQuery({
    queryKey: ['agents'],
    queryFn: fetchAgents,
    refetchInterval: 30_000,
  })

  const addMut = useMutation({
    mutationFn: () =>
      addAgent(form.name.trim(), form.role, form.description.trim() || undefined, form.endpoint.trim() || undefined),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['agents'] })
      setForm({ name: '', role: 'worker', description: '', endpoint: '' })
      setShowAdd(false)
    },
  })

  return (
    <div className="space-y-6">
      <div className="page-header">
        <div>
          <h1 className="page-title">Agents</h1>
          <p className="page-subtitle">{agents.length} agents registered</p>
        </div>
        <button onClick={() => setShowAdd(true)} className="btn-primary">
          <Plus className="w-4 h-4" />
          Register Agent
        </button>
      </div>

      {isLoading ? (
        <PageSpinner />
      ) : agents.length === 0 ? (
        <Empty
          icon={<Bot className="w-8 h-8" />}
          title="No agents registered"
          description="Register sub-agents that Praxis can delegate work to."
          action={
            <button onClick={() => setShowAdd(true)} className="btn-primary">
              <Plus className="w-4 h-4" />
              Register Agent
            </button>
          }
        />
      ) : (
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
          {agents.map((a) => <AgentCard key={a.id} agent={a} />)}
        </div>
      )}

      <Modal open={showAdd} onClose={() => setShowAdd(false)} title="Register Agent">
        <div className="space-y-4">
          <Input
            label="Name"
            value={form.name}
            onChange={(e) => setForm((f) => ({ ...f, name: e.target.value }))}
            placeholder="e.g. hermes-reviewer"
          />
          <div className="space-y-1">
            <label className="text-sm font-medium text-slate-600 dark:text-slate-300">Role</label>
            <select
              value={form.role}
              onChange={(e) => setForm((f) => ({ ...f, role: e.target.value }))}
              className="w-full px-3 py-2 text-sm rounded-lg border border-slate-200 dark:border-slate-700
                         bg-white dark:bg-slate-800 text-slate-700 dark:text-slate-200
                         focus:outline-none focus:ring-2 focus:ring-brand-500/20 focus:border-brand-500"
            >
              <option value="worker">Worker</option>
              <option value="orchestrator">Orchestrator</option>
              <option value="reviewer">Reviewer</option>
              <option value="specialist">Specialist</option>
            </select>
          </div>
          <Textarea
            label="Description (optional)"
            value={form.description}
            onChange={(e) => setForm((f) => ({ ...f, description: e.target.value }))}
            placeholder="What does this agent do?"
            rows={3}
          />
          <Input
            label="Endpoint (optional)"
            value={form.endpoint}
            onChange={(e) => setForm((f) => ({ ...f, endpoint: e.target.value }))}
            placeholder="e.g. http://localhost:9090"
          />
          {addMut.isError && (
            <p className="text-sm text-red-500">
              {addMut.error instanceof Error ? addMut.error.message : 'Failed to register agent'}
            </p>
          )}
          <div className="flex gap-3 justify-end">
            <button onClick={() => setShowAdd(false)} className="btn-secondary">Cancel</button>
            <button
              onClick={() => addMut.mutate()}
              disabled={!form.name.trim() || addMut.isPending}
              className="btn-primary"
            >
              {addMut.isPending ? 'Registering…' : 'Register'}
            </button>
          </div>
        </div>
      </Modal>
    </div>
  )
}
