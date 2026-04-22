import { Moon, Play, Sun, WifiOff, Zap } from 'lucide-react'
import { useState } from 'react'
import { useTheme } from '../../contexts/ThemeContext'
import { useToast } from '../../contexts/ToastContext'
import { triggerRun, triggerWake } from '../../lib/api'
import { useQueryClient } from '@tanstack/react-query'

interface HeaderProps {
  connected: boolean
  sidebarCollapsed: boolean
}

export function Header({ connected, sidebarCollapsed }: HeaderProps) {
  const { theme, toggle } = useTheme()
  const { addToast } = useToast()
  const qc = useQueryClient()
  const [running, setRunning] = useState(false)

  const handleRun = async () => {
    setRunning(true)
    try {
      await triggerRun()
      addToast('Session started', 'success')
      await qc.invalidateQueries()
    } catch (e) {
      addToast(e instanceof Error ? e.message : 'Failed to run session', 'error')
    } finally {
      setRunning(false)
    }
  }

  const handleWake = async () => {
    try {
      await triggerWake(undefined, 'dashboard trigger', true)
      addToast('Wake intent sent', 'success')
    } catch (e) {
      addToast(e instanceof Error ? e.message : 'Failed to send wake intent', 'error')
    }
  }

  return (
    <header
      className="fixed top-0 right-0 left-0 z-10 flex items-center h-14 px-4 gap-3
                 bg-white/80 dark:bg-slate-950/80 backdrop-blur-sm
                 border-b border-slate-200 dark:border-slate-800"
      style={{ paddingLeft: `calc(${sidebarCollapsed ? '3.5rem' : '15rem'} + 1rem)` }}
    >
      {/* Connection status */}
      <div className="flex items-center gap-1.5 text-xs font-medium">
        {connected ? (
          <>
            <span className="live-dot" />
            <span className="text-emerald-600 dark:text-emerald-400 hidden sm:inline">Live</span>
          </>
        ) : (
          <>
            <WifiOff className="w-3.5 h-3.5 text-slate-400" />
            <span className="text-slate-400 hidden sm:inline">Offline</span>
          </>
        )}
      </div>

      <div className="flex-1" />

      {/* Actions */}
      <div className="flex items-center gap-1.5">
        <button
          onClick={handleWake}
          className="btn-ghost text-amber-600 dark:text-amber-400 hover:bg-amber-50 dark:hover:bg-amber-900/20"
          title="Send wake intent"
        >
          <Zap className="w-4 h-4" />
          <span className="hidden sm:inline">Wake</span>
        </button>

        <button
          onClick={handleRun}
          disabled={running}
          className="btn-primary py-1.5 text-xs"
          title="Run one session now"
        >
          <Play className="w-3.5 h-3.5" />
          <span>{running ? 'Running…' : 'Run'}</span>
        </button>

        <button
          onClick={toggle}
          className="p-2 rounded-lg hover:bg-slate-100 dark:hover:bg-slate-800 text-slate-500 dark:text-slate-400 transition-colors"
          title="Toggle theme"
        >
          {theme === 'dark' ? <Sun className="w-4 h-4" /> : <Moon className="w-4 h-4" />}
        </button>
      </div>
    </header>
  )
}
