import { useState } from 'react'
import { Outlet } from 'react-router-dom'
import { useQuery } from '@tanstack/react-query'
import { Sidebar } from './Sidebar'
import { Header } from './Header'
import { useSSE } from '../../hooks/useSSE'
import { fetchApprovals } from '../../lib/api'
import { cn } from '../../lib/utils'

export function Layout() {
  const [collapsed, setCollapsed] = useState(false)
  const { connected } = useSSE()

  const { data: approvals } = useQuery({
    queryKey: ['approvals'],
    queryFn: fetchApprovals,
    refetchInterval: 30_000,
  })

  const pendingCount = approvals?.filter((a) => a.status === 'pending').length ?? 0

  return (
    <div className="min-h-screen bg-slate-50 dark:bg-slate-950">
      <Sidebar
        pendingApprovals={pendingCount}
        collapsed={collapsed}
        onCollapse={() => setCollapsed((c) => !c)}
      />
      <Header connected={connected} sidebarCollapsed={collapsed} />
      <main
        className={cn(
          'pt-14 min-h-screen transition-all duration-200',
          collapsed ? 'pl-14' : 'pl-60',
        )}
      >
        <div className="p-6 max-w-7xl mx-auto animate-fade-in">
          <Outlet />
        </div>
      </main>
    </div>
  )
}
