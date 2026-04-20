import { useState, useEffect } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { Save, Settings } from 'lucide-react'
import { fetchConfig, updateConfig } from '../lib/api'
import { Card } from '../components/ui/Card'
import { PageSpinner } from '../components/ui/Spinner'

type ConfigSection = Record<string, unknown>

function renderValue(v: unknown): string {
  if (typeof v === 'object' && v !== null) return JSON.stringify(v, null, 2)
  return String(v ?? '')
}

function ConfigField({
  label,
  value,
  onChange,
  readOnly,
}: {
  label: string
  value: string
  onChange: (v: string) => void
  readOnly?: boolean
}) {
  const isMultiline = value.includes('\n') || value.length > 80

  return (
    <div className="space-y-1">
      <label className="text-xs font-mono font-medium text-slate-500 dark:text-slate-400 uppercase tracking-wider">
        {label}
      </label>
      {isMultiline ? (
        <textarea
          value={value}
          onChange={(e) => onChange(e.target.value)}
          readOnly={readOnly}
          rows={Math.min(value.split('\n').length + 1, 12)}
          className="w-full px-3 py-2 text-xs font-mono rounded-lg border border-slate-200 dark:border-slate-700
                     bg-slate-50 dark:bg-slate-800 text-slate-700 dark:text-slate-300
                     focus:outline-none focus:ring-2 focus:ring-brand-500/20 focus:border-brand-500
                     resize-y disabled:opacity-60 disabled:cursor-not-allowed"
          disabled={readOnly}
        />
      ) : (
        <input
          type="text"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          readOnly={readOnly}
          className="w-full px-3 py-2 text-xs font-mono rounded-lg border border-slate-200 dark:border-slate-700
                     bg-slate-50 dark:bg-slate-800 text-slate-700 dark:text-slate-300
                     focus:outline-none focus:ring-2 focus:ring-brand-500/20 focus:border-brand-500
                     disabled:opacity-60 disabled:cursor-not-allowed"
          disabled={readOnly}
        />
      )}
    </div>
  )
}

export function Config() {
  const [draft, setDraft] = useState<Record<string, string>>({})
  const [dirty, setDirty] = useState(false)
  const qc = useQueryClient()

  const { data: config, isLoading } = useQuery({
    queryKey: ['config'],
    queryFn: fetchConfig,
  })

  useEffect(() => {
    if (config) {
      const flat: Record<string, string> = {}
      for (const [section, values] of Object.entries(config)) {
        if (typeof values === 'object' && values !== null) {
          for (const [key, val] of Object.entries(values as ConfigSection)) {
            flat[`${section}.${key}`] = renderValue(val)
          }
        } else {
          flat[section] = renderValue(values)
        }
      }
      setDraft(flat)
      setDirty(false)
    }
  }, [config])

  const saveMut = useMutation({
    mutationFn: () => updateConfig(draft),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['config'] })
      setDirty(false)
    },
  })

  const sections = new Map<string, Array<[string, string]>>()
  for (const [key, val] of Object.entries(draft)) {
    const dot = key.indexOf('.')
    const section = dot !== -1 ? key.slice(0, dot) : 'general'
    const field = dot !== -1 ? key.slice(dot + 1) : key
    if (!sections.has(section)) sections.set(section, [])
    sections.get(section)!.push([field, val])
  }

  const READONLY_FIELDS = new Set(['data_dir', 'version', 'database_path'])

  return (
    <div className="space-y-6">
      <div className="page-header">
        <div>
          <h1 className="page-title">Config</h1>
          <p className="page-subtitle">Runtime configuration (praxis.toml)</p>
        </div>
        {dirty && (
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

      {isLoading ? (
        <PageSpinner />
      ) : (
        <div className="space-y-4">
          {Array.from(sections.entries()).map(([section, fields]) => (
            <Card key={section} padding="none">
              <div className="px-5 py-3 border-b border-slate-100 dark:border-slate-800 flex items-center gap-2">
                <Settings className="w-3.5 h-3.5 text-slate-400" />
                <span className="text-sm font-mono font-semibold text-slate-700 dark:text-slate-300 capitalize">
                  {section}
                </span>
              </div>
              <div className="p-5 grid grid-cols-1 sm:grid-cols-2 gap-4">
                {fields.map(([field, val]) => {
                  const fullKey = section === 'general' ? field : `${section}.${field}`
                  return (
                    <ConfigField
                      key={fullKey}
                      label={field}
                      value={val}
                      readOnly={READONLY_FIELDS.has(field)}
                      onChange={(v) => {
                        setDraft((d) => ({ ...d, [fullKey]: v }))
                        setDirty(true)
                      }}
                    />
                  )
                })}
              </div>
            </Card>
          ))}

          {saveMut.isError && (
            <p className="text-sm text-red-500">
              {saveMut.error instanceof Error ? saveMut.error.message : 'Failed to save config'}
            </p>
          )}
        </div>
      )}
    </div>
  )
}
