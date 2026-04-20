import { useState, useEffect } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { FileText, Save } from 'lucide-react'
import { fetchIdentityFile, writeIdentityFile } from '../lib/api'
import { Card } from '../components/ui/Card'
import { PageSpinner } from '../components/ui/Spinner'

const FILES = [
  { key: 'identity', label: 'IDENTITY.md', writable: true },
  { key: 'soul', label: 'SOUL.md', writable: false },
  { key: 'goals', label: 'GOALS.md', writable: true },
  { key: 'agents', label: 'AGENTS.md', writable: true },
  { key: 'journal', label: 'JOURNAL.md', writable: true },
  { key: 'patterns', label: 'PATTERNS.md', writable: true },
  { key: 'learnings', label: 'LEARNINGS.md', writable: true },
  { key: 'roadmap', label: 'ROADMAP.md', writable: true },
] as const

type FileKey = (typeof FILES)[number]['key']

export function Identity() {
  const [activeFile, setActiveFile] = useState<FileKey>('identity')
  const [draft, setDraft] = useState('')
  const [dirty, setDirty] = useState(false)
  const qc = useQueryClient()

  const { data, isLoading } = useQuery({
    queryKey: ['identity', activeFile],
    queryFn: () => fetchIdentityFile(activeFile),
  })

  useEffect(() => {
    if (data) {
      setDraft(data.content)
      setDirty(false)
    }
  }, [data])

  const saveMut = useMutation({
    mutationFn: () => writeIdentityFile(activeFile, draft),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['identity', activeFile] })
      setDirty(false)
    },
  })

  const fileInfo = FILES.find((f) => f.key === activeFile)

  return (
    <div className="space-y-6">
      <div className="page-header">
        <div>
          <h1 className="page-title">Identity</h1>
          <p className="page-subtitle">Agent identity and foundational documents</p>
        </div>
        {dirty && fileInfo?.writable && (
          <button
            onClick={() => saveMut.mutate()}
            disabled={saveMut.isPending}
            className="btn-primary"
          >
            <Save className="w-4 h-4" />
            {saveMut.isPending ? 'Saving…' : 'Save'}
          </button>
        )}
      </div>

      <div className="flex gap-6">
        {/* File picker */}
        <div className="flex-shrink-0 w-48 space-y-1">
          {FILES.map((f) => (
            <button
              key={f.key}
              onClick={() => {
                if (dirty && !confirm('Discard unsaved changes?')) return
                setActiveFile(f.key)
              }}
              className={`w-full text-left px-3 py-2 rounded-lg text-sm font-mono transition-all ${
                activeFile === f.key
                  ? 'bg-brand-50 dark:bg-brand-900/20 text-brand-700 dark:text-brand-400 font-medium'
                  : 'text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800'
              }`}
            >
              <div className="flex items-center gap-2">
                <FileText className="w-3.5 h-3.5 flex-shrink-0" />
                <span className="truncate">{f.label}</span>
              </div>
              {!f.writable && (
                <span className="text-xs text-slate-400 mt-0.5 block ml-5.5">read-only</span>
              )}
            </button>
          ))}
        </div>

        {/* Editor */}
        <div className="flex-1 min-w-0">
          {isLoading ? (
            <PageSpinner />
          ) : (
            <Card padding="none" className="overflow-hidden">
              <div className="flex items-center justify-between px-4 py-3 border-b border-slate-100 dark:border-slate-800">
                <span className="text-sm font-mono font-medium text-slate-600 dark:text-slate-300">
                  {fileInfo?.label}
                </span>
                {dirty && (
                  <span className="text-xs text-amber-500 font-medium">● Unsaved</span>
                )}
                {!fileInfo?.writable && (
                  <span className="badge bg-slate-100 dark:bg-slate-800 text-slate-500">
                    read-only
                  </span>
                )}
              </div>
              <textarea
                value={draft}
                onChange={(e) => {
                  setDraft(e.target.value)
                  setDirty(true)
                }}
                readOnly={!fileInfo?.writable}
                className="w-full h-[calc(100vh-20rem)] p-4 text-xs font-mono
                           bg-transparent text-slate-800 dark:text-slate-200
                           resize-none focus:outline-none leading-relaxed"
                spellCheck={false}
              />
            </Card>
          )}
        </div>
      </div>
    </div>
  )
}
