import { useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Eye, EyeOff, KeyRound, Plus, Trash2 } from 'lucide-react'
import { fetchVault, setVaultSecret, deleteVaultSecret, type VaultEntry } from '../lib/api'
import { Card } from '../components/ui/Card'
import { PageSpinner } from '../components/ui/Spinner'
import { Empty } from '../components/ui/Empty'
import { Modal } from '../components/ui/Modal'
import { Input } from '../components/ui/Input'

function VaultRow({ entry, onDelete }: { entry: VaultEntry; onDelete: () => void }) {
  const [revealed, setRevealed] = useState(false)

  return (
    <div className="flex items-center gap-3 py-3 border-b border-slate-100 dark:border-slate-800 last:border-0 group">
      <KeyRound className="w-4 h-4 text-slate-400 flex-shrink-0" />
      <div className="flex-1 min-w-0">
        <p className="text-sm font-mono font-semibold text-slate-800 dark:text-slate-200">{entry.key}</p>
        <p className="text-xs font-mono text-slate-400 mt-0.5">
          {revealed ? entry.value : '••••••••••••'}
        </p>
      </div>
      <div className="flex-shrink-0 flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
        <button
          onClick={() => setRevealed((r) => !r)}
          className="btn-ghost text-slate-400 hover:text-slate-600 dark:hover:text-slate-200"
          title={revealed ? 'Hide' : 'Reveal'}
        >
          {revealed ? <EyeOff className="w-3.5 h-3.5" /> : <Eye className="w-3.5 h-3.5" />}
        </button>
        <button
          onClick={onDelete}
          className="btn-ghost text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20"
          title="Delete"
        >
          <Trash2 className="w-3.5 h-3.5" />
        </button>
      </div>
    </div>
  )
}

export function Vault() {
  const [showAdd, setShowAdd] = useState(false)
  const [form, setForm] = useState({ key: '', value: '' })
  const [showValue, setShowValue] = useState(false)
  const qc = useQueryClient()

  const { data: entries = [], isLoading } = useQuery({
    queryKey: ['vault'],
    queryFn: fetchVault,
  })

  const setMut = useMutation({
    mutationFn: () => setVaultSecret(form.key.trim(), form.value),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['vault'] })
      setForm({ key: '', value: '' })
      setShowAdd(false)
    },
  })

  const deleteMut = useMutation({
    mutationFn: (key: string) => deleteVaultSecret(key),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['vault'] }),
  })

  return (
    <div className="space-y-6">
      <div className="page-header">
        <div>
          <h1 className="page-title">Vault</h1>
          <p className="page-subtitle">Encrypted secrets store (AES-256-GCM)</p>
        </div>
        <button onClick={() => setShowAdd(true)} className="btn-primary">
          <Plus className="w-4 h-4" />
          Add Secret
        </button>
      </div>

      <div className="px-4 py-3 bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 rounded-xl text-sm text-amber-700 dark:text-amber-300">
        Secrets are stored encrypted at rest. Values are revealed only on hover and are never logged.
      </div>

      {isLoading ? (
        <PageSpinner />
      ) : entries.length === 0 ? (
        <Empty
          icon={<KeyRound className="w-8 h-8" />}
          title="No secrets stored"
          description="Store API keys and credentials securely in the encrypted vault."
          action={
            <button onClick={() => setShowAdd(true)} className="btn-primary">
              <Plus className="w-4 h-4" />
              Add Secret
            </button>
          }
        />
      ) : (
        <Card padding="none">
          <div className="px-5 divide-y divide-slate-100 dark:divide-slate-800">
            {entries.map((e) => (
              <VaultRow
                key={e.key}
                entry={e}
                onDelete={() => deleteMut.mutate(e.key)}
              />
            ))}
          </div>
        </Card>
      )}

      <Modal open={showAdd} onClose={() => setShowAdd(false)} title="Add Secret">
        <div className="space-y-4">
          <Input
            label="Key"
            value={form.key}
            onChange={(e) => setForm((f) => ({ ...f, key: e.target.value }))}
            placeholder="e.g. OPENAI_API_KEY"
          />
          <div className="space-y-1">
            <label className="text-sm font-medium text-slate-600 dark:text-slate-300">Value</label>
            <div className="relative">
              <input
                type={showValue ? 'text' : 'password'}
                value={form.value}
                onChange={(e) => setForm((f) => ({ ...f, value: e.target.value }))}
                placeholder="Secret value"
                className="w-full px-3 py-2 pr-10 text-sm rounded-lg border border-slate-200 dark:border-slate-700
                           bg-white dark:bg-slate-800 text-slate-700 dark:text-slate-200
                           focus:outline-none focus:ring-2 focus:ring-brand-500/20 focus:border-brand-500"
              />
              <button
                type="button"
                onClick={() => setShowValue((v) => !v)}
                className="absolute right-2.5 top-1/2 -translate-y-1/2 text-slate-400 hover:text-slate-600 dark:hover:text-slate-200"
              >
                {showValue ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
              </button>
            </div>
          </div>
          {setMut.isError && (
            <p className="text-sm text-red-500">
              {setMut.error instanceof Error ? setMut.error.message : 'Failed to store secret'}
            </p>
          )}
          <div className="flex gap-3 justify-end">
            <button onClick={() => setShowAdd(false)} className="btn-secondary">Cancel</button>
            <button
              onClick={() => setMut.mutate()}
              disabled={!form.key.trim() || !form.value || setMut.isPending}
              className="btn-primary"
            >
              {setMut.isPending ? 'Storing…' : 'Store Secret'}
            </button>
          </div>
        </div>
      </Modal>
    </div>
  )
}
