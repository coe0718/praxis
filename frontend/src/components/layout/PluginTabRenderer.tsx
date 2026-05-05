import { useEffect, useRef, useState } from 'react'
import { AlertTriangle, Loader2 } from 'lucide-react'

interface Props {
  contentUrl: string
}

export function PluginTabRenderer({ contentUrl }: Props) {
  const iframeRef = useRef<HTMLIFrameElement>(null)
  const [status, setStatus] = useState<'loading' | 'loaded' | 'error'>('loading')

  useEffect(() => {
    setStatus('loading')
    const iframe = iframeRef.current
    if (!iframe) return
    iframe.onload = () => setStatus('loaded')
    iframe.onerror = () => setStatus('error')
    // Set src after setting handlers to avoid race
    iframe.src = contentUrl
  }, [contentUrl])

  return (
    <div className="relative h-full min-h-[400px]">
      <iframe
        ref={iframeRef}
        className="absolute inset-0 w-full h-full border-0 rounded-b-xl"
        title="plugin content"
        sandbox="allow-scripts allow-same-origin allow-forms allow-popups"
      />
      {status === 'loading' && (
        <div className="absolute inset-0 flex items-center justify-center bg-slate-50/80 dark:bg-slate-900/80 rounded-xl">
          <Loader2 className="w-6 h-6 animate-spin text-brand-500" />
        </div>
      )}
      {status === 'error' && (
        <div className="absolute inset-0 flex flex-col items-center justify-center gap-2 text-red-500 bg-slate-50 dark:bg-slate-900 rounded-xl">
          <AlertTriangle className="w-6 h-6" />
          <span className="text-sm">Failed to load plugin content</span>
        </div>
      )}
    </div>
  )
}