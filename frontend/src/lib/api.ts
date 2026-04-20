const getBaseUrl = (): string =>
  localStorage.getItem('praxis_base_url') ?? ''

const getToken = (): string | null =>
  localStorage.getItem('praxis_token')

function headers(): HeadersInit {
  const token = getToken()
  const h: Record<string, string> = { 'Content-Type': 'application/json' }
  if (token) h['Authorization'] = `Bearer ${token}`
  return h
}

async function request<T>(
  path: string,
  options: RequestInit = {},
): Promise<T> {
  const res = await fetch(`${getBaseUrl()}${path}`, {
    ...options,
    headers: { ...headers(), ...(options.headers ?? {}) },
  })
  if (!res.ok) {
    const text = await res.text().catch(() => res.statusText)
    throw new Error(`${res.status}: ${text}`)
  }
  return res.json() as Promise<T>
}

// ── Summary & status ──────────────────────────────────────────────────────────

export const fetchSummary = (): Promise<Record<string, unknown>> =>
  request('/api/summary')

export const fetchReport = (): Promise<{ report: string }> =>
  request('/api/report')

export const fetchHeartbeat = (): Promise<Record<string, unknown>> =>
  request('/api/heartbeat')

// ── Sessions ──────────────────────────────────────────────────────────────────

export interface SessionRow {
  id: number
  day: number
  session_num: number
  started_at: string
  ended_at: string
  outcome: string
  selected_goal_id: string | null
  selected_goal_title: string | null
  selected_task: string | null
  action_summary: string
}

export const fetchSessions = (): Promise<SessionRow[]> =>
  request('/api/sessions')

// ── Goals ─────────────────────────────────────────────────────────────────────

export interface Goal {
  raw_id: string
  title: string
  completed: boolean
}

export const fetchGoals = (): Promise<Goal[]> => request('/api/goals')
export const addGoal = (description: string): Promise<{ id: string; title: string }> =>
  request('/api/goals', {
    method: 'POST',
    body: JSON.stringify({ description }),
  })

// ── Approvals ─────────────────────────────────────────────────────────────────

export interface Approval {
  id: number
  tool_name: string
  summary: string
  requested_by: string
  write_paths: string[]
  payload_json: string | null
  status: string
  status_note: string | null
  created_at: string
  updated_at: string
}

export const fetchApprovals = (): Promise<Approval[]> => request('/api/approvals')
export const approveRequest = (id: number): Promise<{ id: number; status: string }> =>
  request(`/api/approvals/${id}/approve`, { method: 'POST' })
export const rejectRequest = (id: number): Promise<{ id: number; status: string }> =>
  request(`/api/approvals/${id}/reject`, { method: 'POST' })

// ── Memories ──────────────────────────────────────────────────────────────────

export interface Memory {
  id: number
  tier: string
  content: string
  summary: string | null
  tags: string[]
  score: number
  memory_type: string
}

export const fetchHotMemories = (): Promise<Memory[]> => request('/api/memories/hot')
export const fetchColdMemories = (): Promise<Memory[]> => request('/api/memories/cold')
export const reinforceMemory = (id: number): Promise<{ id: number; action: string }> =>
  request(`/api/memories/${id}/reinforce`, { method: 'POST' })
export const forgetMemory = (id: number): Promise<{ id: number; action: string }> =>
  request(`/api/memories/${id}/forget`, { method: 'POST' })
export const consolidateMemories = (): Promise<{ consolidated: number; pruned: number }> =>
  request('/api/memories/consolidate', { method: 'POST' })

// ── Tools ─────────────────────────────────────────────────────────────────────

export interface Tool {
  name: string
  description: string
  kind: string
  required_level: number
  requires_approval: boolean
  cooldown_seconds?: number
  allowed_paths?: string[]
}

export const fetchTools = (): Promise<Tool[]> => request('/api/tools')

// ── Identity ──────────────────────────────────────────────────────────────────

export const fetchIdentityFile = (file: string): Promise<{ file: string; content: string; writable: boolean }> =>
  request(`/api/identity/${file}`)
export const writeIdentityFile = (file: string, content: string): Promise<{ file: string; saved: boolean }> =>
  request(`/api/identity/${file}`, {
    method: 'PUT',
    body: JSON.stringify({ content }),
  })

// ── Config ────────────────────────────────────────────────────────────────────

export const fetchConfig = (): Promise<Record<string, unknown>> =>
  request('/api/config')

export const updateConfig = (fields: Record<string, string>): Promise<{ saved: boolean }> =>
  request('/api/config', {
    method: 'PUT',
    body: JSON.stringify(fields),
  })

// ── Canary ────────────────────────────────────────────────────────────────────

export interface CanaryResult {
  test_name: string
  passed: boolean
  message?: string
  latency_ms?: number
}

export const runCanary = (provider?: string): Promise<CanaryResult[]> =>
  request('/api/canary/run', {
    method: 'POST',
    body: JSON.stringify({ provider }),
  })

// ── Score ─────────────────────────────────────────────────────────────────────

export interface SessionScore {
  session_id: number
  composite: number
  anticipation: number
  follow_through: number
  reliability: number
  independence: number
  created_at: string
}

export const fetchScores = (): Promise<SessionScore[]> => request('/api/score')

// ── Evolution ─────────────────────────────────────────────────────────────────

export interface EvolutionProposal {
  id: string
  proposal_type: string
  title: string
  rationale: string
  status: string
  proposed_change?: string
  session_id?: number
  created_at: string
}

export const fetchEvolution = (): Promise<EvolutionProposal[]> => request('/api/evolution')
export const approveEvolution = (id: string): Promise<{ id: string; approved: boolean; status: string }> =>
  request(`/api/evolution/${id}/approve`, { method: 'POST' })

// ── Delegation ────────────────────────────────────────────────────────────────

export interface DelegationLink {
  id: number
  from_agent: string
  to_agent: string
  task_summary?: string
  result_summary?: string
  error?: string
  status: string
  created_at: string
}

export const fetchDelegation = (): Promise<DelegationLink[]> =>
  request('/api/delegation')

// ── Wake & Run ────────────────────────────────────────────────────────────────

export const triggerWake = (task?: string, reason?: string, urgent?: boolean): Promise<{ queued: boolean; reason: string }> =>
  request('/api/wake', {
    method: 'POST',
    body: JSON.stringify({ task, reason, urgent }),
  })

export const triggerRun = (task?: string): Promise<{ outcome: string }> =>
  request('/api/run', {
    method: 'POST',
    body: JSON.stringify({ task }),
  })

// ── Chat (Ask) ────────────────────────────────────────────────────────────────

export const sendAsk = (prompt: string): Promise<{ output: string }> =>
  request('/api/ask', {
    method: 'POST',
    body: JSON.stringify({ prompt }),
  })

// ── Boundaries ────────────────────────────────────────────────────────────────

export interface Boundary {
  id: number
  description: string
  category: string
  rationale?: string
  source?: string
  confirmed: boolean
  created_at: string
}

export const fetchBoundaries = (): Promise<Boundary[]> => request('/api/boundaries')
export const addBoundary = (description: string, category: string, rationale?: string): Promise<{ added: boolean }> =>
  request('/api/boundaries', {
    method: 'POST',
    body: JSON.stringify({ description, category, rationale }),
  })
export const confirmBoundary = (id: number): Promise<{ confirmed: boolean }> =>
  request(`/api/boundaries/${id}/confirm`, { method: 'POST' })
export const removeBoundary = (id: number): Promise<{ deleted: boolean }> =>
  request(`/api/boundaries/${id}`, { method: 'DELETE' })

// ── Forensics ─────────────────────────────────────────────────────────────────

export interface ForensicsAnomaly {
  description: string
  context?: string
  severity?: string
  detected_at: string
}

export interface ForensicsSystemSnapshot {
  captured_at?: string
  memory_mb?: number
  cpu_percent?: number
  open_files?: number
  db_size_kb?: number
}

export interface ForensicsReport {
  anomalies?: ForensicsAnomaly[]
  snapshot?: ForensicsSystemSnapshot
  [key: string]: unknown
}

export const fetchForensics = (): Promise<ForensicsReport> => request('/api/forensics')

// ── Argus ─────────────────────────────────────────────────────────────────────

export interface ArgusRepeatedWork {
  label: string
  sessions: number
  distinct_days: number
  latest_outcome: string
}

export interface ArgusTokenHotspot {
  provider: string
  model: string
  tokens: number
}

export interface ArgusReport {
  review_failures?: string[]
  eval_failures?: string[]
  drift_status?: string
  drift_recent_score?: number
  drift_baseline_score?: number
  repeated_work?: ArgusRepeatedWork[]
  token_hotspots?: ArgusTokenHotspot[]
  report_text?: string
  [key: string]: unknown
}

export const fetchArgus = (): Promise<ArgusReport> => request('/api/argus')

// ── Learning ──────────────────────────────────────────────────────────────────

export interface LearningOpportunity {
  id?: number
  title: string
  description: string
  category: string
  priority: string
  source?: string
  created_at: string
}

export const fetchLearningOpportunities = (): Promise<LearningOpportunity[]> =>
  request('/api/learning/opportunities')
export const triggerLearningRun = (): Promise<{ opportunities_found?: number; notes?: string[] }> =>
  request('/api/learning/run', { method: 'POST' })
export const addLearningNote = (text: string): Promise<{ added: boolean; summary: string }> =>
  request('/api/learning/note', {
    method: 'POST',
    body: JSON.stringify({ text }),
  })

// ── Agents ────────────────────────────────────────────────────────────────────

export interface AgentEntry {
  id: number
  name: string
  role?: string
  description?: string
  endpoint?: string
  capabilities?: string[]
  created_at: string
}

export const fetchAgents = (): Promise<AgentEntry[]> => request('/api/agents')
export const addAgent = (
  name: string,
  role: string,
  description?: string,
  endpoint?: string,
): Promise<{ id: number; name: string }> =>
  request('/api/agents', {
    method: 'POST',
    body: JSON.stringify({ name, role, description, endpoint }),
  })

// ── Vault ─────────────────────────────────────────────────────────────────────

export interface VaultEntry {
  key: string
  value: string
}

export const fetchVault = (): Promise<VaultEntry[]> => request('/api/vault')
export const setVaultSecret = (key: string, value: string): Promise<{ saved: boolean }> =>
  request('/api/vault', {
    method: 'POST',
    body: JSON.stringify({ key, value }),
  })
export const deleteVaultSecret = (key: string): Promise<{ deleted: boolean }> =>
  request(`/api/vault/${encodeURIComponent(key)}`, { method: 'DELETE' })

// ── Events (SSE) ──────────────────────────────────────────────────────────────

export interface PraxisEvent {
  kind: string
  detail: string
  at: string
}

export const fetchRecentEvents = (): Promise<PraxisEvent[]> => request('/events/recent')
