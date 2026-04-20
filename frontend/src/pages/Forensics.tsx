import { useQuery } from '@tanstack/react-query'
import { Search, AlertTriangle, Clock } from 'lucide-react'
import { fetchForensics, type ForensicsReport } from '../lib/api'
import { Card } from '../components/ui/Card'
import { PageSpinner } from '../components/ui/Spinner'
import { Empty } from '../components/ui/Empty'
import { formatDate } from '../lib/utils'

function AnomalySection({ report }: { report: ForensicsReport }) {
  if (!report.anomalies || report.anomalies.length === 0) return null

  return (
    <Card padding="none">
      <div className="px-5 py-4 border-b border-slate-100 dark:border-slate-800 flex items-center gap-2">
        <AlertTriangle className="w-4 h-4 text-amber-500" />
        <h2 className="text-sm font-semibold text-slate-700 dark:text-slate-300">
          Anomalies ({report.anomalies.length})
        </h2>
      </div>
      <div className="divide-y divide-slate-100 dark:divide-slate-800">
        {report.anomalies.map((a, i) => (
          <div key={i} className="px-5 py-3.5">
            <div className="flex items-start gap-3">
              <div className="flex-1 min-w-0">
                <p className="text-sm font-medium text-slate-800 dark:text-slate-200">{a.description}</p>
                {a.context && (
                  <p className="mt-0.5 text-xs text-slate-500 dark:text-slate-400">{a.context}</p>
                )}
                <div className="mt-1 flex items-center gap-2 text-xs text-slate-400">
                  <Clock className="w-3 h-3" />
                  <span>{formatDate(a.detected_at)}</span>
                  {a.severity && (
                    <span className={`font-medium ${
                      a.severity === 'high' ? 'text-red-500' :
                      a.severity === 'medium' ? 'text-amber-500' :
                      'text-slate-400'
                    }`}>
                      {a.severity}
                    </span>
                  )}
                </div>
              </div>
            </div>
          </div>
        ))}
      </div>
    </Card>
  )
}

function SystemSnapshotSection({ report }: { report: ForensicsReport }) {
  if (!report.snapshot) return null
  const snap = report.snapshot

  return (
    <Card padding="none">
      <div className="px-5 py-4 border-b border-slate-100 dark:border-slate-800">
        <h2 className="text-sm font-semibold text-slate-700 dark:text-slate-300">System Snapshot</h2>
        {snap.captured_at && (
          <p className="text-xs text-slate-400 mt-0.5">{formatDate(snap.captured_at)}</p>
        )}
      </div>
      <div className="p-5 grid grid-cols-2 sm:grid-cols-4 gap-4 text-sm">
        {snap.memory_mb !== undefined && (
          <div>
            <p className="text-xs text-slate-400 uppercase tracking-wider mb-0.5">Memory</p>
            <p className="font-mono text-slate-700 dark:text-slate-300">{snap.memory_mb} MB</p>
          </div>
        )}
        {snap.cpu_percent !== undefined && (
          <div>
            <p className="text-xs text-slate-400 uppercase tracking-wider mb-0.5">CPU</p>
            <p className="font-mono text-slate-700 dark:text-slate-300">{snap.cpu_percent}%</p>
          </div>
        )}
        {snap.open_files !== undefined && (
          <div>
            <p className="text-xs text-slate-400 uppercase tracking-wider mb-0.5">Open Files</p>
            <p className="font-mono text-slate-700 dark:text-slate-300">{snap.open_files}</p>
          </div>
        )}
        {snap.db_size_kb !== undefined && (
          <div>
            <p className="text-xs text-slate-400 uppercase tracking-wider mb-0.5">DB Size</p>
            <p className="font-mono text-slate-700 dark:text-slate-300">{snap.db_size_kb} KB</p>
          </div>
        )}
      </div>
    </Card>
  )
}

function RawJsonSection({ data }: { data: unknown }) {
  return (
    <Card padding="none">
      <div className="px-5 py-4 border-b border-slate-100 dark:border-slate-800">
        <h2 className="text-sm font-semibold text-slate-700 dark:text-slate-300">Raw Report</h2>
      </div>
      <pre className="p-5 text-xs font-mono text-slate-600 dark:text-slate-400 overflow-x-auto whitespace-pre-wrap">
        {JSON.stringify(data, null, 2)}
      </pre>
    </Card>
  )
}

export function Forensics() {
  const { data: report, isLoading } = useQuery({
    queryKey: ['forensics'],
    queryFn: fetchForensics,
    refetchInterval: 60_000,
  })

  return (
    <div className="space-y-6">
      <div className="page-header">
        <div>
          <h1 className="page-title">Forensics</h1>
          <p className="page-subtitle">System snapshots and anomaly detection</p>
        </div>
      </div>

      {isLoading ? (
        <PageSpinner />
      ) : !report ? (
        <Empty
          icon={<Search className="w-8 h-8" />}
          title="No forensics data"
          description="Forensics data is captured during the reflect phase."
        />
      ) : (
        <div className="space-y-4">
          <AnomalySection report={report} />
          <SystemSnapshotSection report={report} />
          <RawJsonSection data={report} />
        </div>
      )}
    </div>
  )
}
