import { NavLink } from 'react-router-dom'
import {
  Activity,
  Bot,
  Brain,
  CheckCircle2,
  ChevronRight,
  Cpu,
  Fingerprint,
  FlaskConical,
  GitBranch,
  Globe,
  GraduationCap,
  Key,
  Layers,
  MessageSquare,
  Radar,
  Settings,
  Shield,
  Sparkles,
  TrendingUp,
  Wrench,
  Zap,
  DollarSign,
} from 'lucide-react'
import { cn } from '../../lib/utils'

interface SidebarProps {
  pendingApprovals: number
  collapsed: boolean
  onCollapse: () => void
}

interface NavItem {
  to: string
  label: string
  icon: React.ReactNode
  badge?: number
}

interface NavGroup {
  label: string
  items: NavItem[]
}

const navGroups: NavGroup[] = [
  {
    label: 'Core',
    items: [
      { to: '/dashboard', label: 'Dashboard', icon: <Activity className="w-4 h-4" /> },
      { to: '/chat', label: 'Chat', icon: <MessageSquare className="w-4 h-4" /> },
      { to: '/sessions', label: 'Sessions', icon: <Layers className="w-4 h-4" /> },
      { to: '/goals', label: 'Goals', icon: <CheckCircle2 className="w-4 h-4" /> },
      { to: '/approvals', label: 'Approvals', icon: <Shield className="w-4 h-4" /> },
    ],
  },
  {
    label: 'Memory & Knowledge',
    items: [
      { to: '/memories', label: 'Memories', icon: <Brain className="w-4 h-4" /> },
      { to: '/identity', label: 'Identity', icon: <Fingerprint className="w-4 h-4" /> },
      { to: '/learning', label: 'Learning', icon: <GraduationCap className="w-4 h-4" /> },
      { to: '/agents', label: 'Agent Notes', icon: <Bot className="w-4 h-4" /> },
    ],
  },
  {
    label: 'Operations',
    items: [
      { to: '/tools', label: 'Tools', icon: <Wrench className="w-4 h-4" /> },
      { to: '/evolution', label: 'Evolution', icon: <Sparkles className="w-4 h-4" /> },
      { to: '/score', label: 'Score', icon: <TrendingUp className="w-4 h-4" /> },
      { to: '/tokens', label: 'Tokens', icon: <DollarSign className="w-4 h-4" /> },
      { to: '/canary', label: 'Canary', icon: <FlaskConical className="w-4 h-4" /> },
      { to: '/delegation', label: 'Delegation', icon: <GitBranch className="w-4 h-4" /> },
    ],
  },
  {
    label: 'Analysis',
    items: [
      { to: '/argus', label: 'Argus', icon: <Radar className="w-4 h-4" /> },
      { to: '/forensics', label: 'Forensics', icon: <Zap className="w-4 h-4" /> },
    ],
  },
  {
    label: 'Admin',
    items: [
      { to: '/boundaries', label: 'Boundaries', icon: <Globe className="w-4 h-4" /> },
      { to: '/vault', label: 'Vault', icon: <Key className="w-4 h-4" /> },
      { to: '/config', label: 'Config', icon: <Settings className="w-4 h-4" /> },
    ],
  },
]

export function Sidebar({ pendingApprovals, collapsed, onCollapse }: SidebarProps) {
  return (
    <aside
      className={cn(
        'fixed left-0 top-0 bottom-0 z-20 flex flex-col',
        'bg-white dark:bg-slate-900 border-r border-slate-200 dark:border-slate-800',
        'transition-all duration-200',
        collapsed ? 'w-14' : 'w-60',
      )}
    >
      {/* Logo */}
      <div className="flex items-center gap-3 px-4 h-14 border-b border-slate-200 dark:border-slate-800">
        <div className="flex-shrink-0 w-7 h-7 rounded-lg bg-gradient-to-br from-brand-500 to-violet-500 flex items-center justify-center">
          <Cpu className="w-4 h-4 text-white" />
        </div>
        {!collapsed && (
          <span className="font-semibold text-slate-900 dark:text-slate-100 tracking-tight">
            Praxis
          </span>
        )}
        <button
          onClick={onCollapse}
          className={cn(
            'ml-auto p-1 rounded-lg hover:bg-slate-100 dark:hover:bg-slate-800 text-slate-400 transition-colors',
            collapsed && 'hidden',
          )}
        >
          <ChevronRight className={cn('w-3.5 h-3.5 transition-transform', collapsed && 'rotate-180')} />
        </button>
      </div>

      {/* Nav */}
      <nav className="flex-1 overflow-y-auto py-3 px-2 space-y-5">
        {navGroups.map((group) => (
          <div key={group.label}>
            {!collapsed && (
              <p className="px-2 mb-1 text-xs font-semibold text-slate-400 dark:text-slate-500 uppercase tracking-wider">
                {group.label}
              </p>
            )}
            <ul className="space-y-0.5">
              {group.items.map((item) => (
                <li key={item.to}>
                  <NavLink
                    to={item.to}
                    className={({ isActive }) =>
                      cn(
                        'flex items-center gap-2.5 px-2 py-2 rounded-lg text-sm font-medium transition-all',
                        'hover:bg-slate-100 dark:hover:bg-slate-800',
                        isActive
                          ? 'bg-brand-50 dark:bg-brand-900/20 text-brand-700 dark:text-brand-400'
                          : 'text-slate-600 dark:text-slate-400',
                        collapsed && 'justify-center',
                      )
                    }
                    title={collapsed ? item.label : undefined}
                  >
                    <span className="flex-shrink-0">{item.icon}</span>
                    {!collapsed && (
                      <>
                        <span className="flex-1">{item.label}</span>
                        {item.to === '/approvals' && pendingApprovals > 0 && (
                          <span className="flex-shrink-0 min-w-[1.25rem] h-5 px-1 rounded-full bg-amber-500 text-white text-xs font-bold flex items-center justify-center">
                            {pendingApprovals > 99 ? '99+' : pendingApprovals}
                          </span>
                        )}
                      </>
                    )}
                    {collapsed && item.to === '/approvals' && pendingApprovals > 0 && (
                      <span className="absolute top-1 right-1 w-2 h-2 rounded-full bg-amber-500" />
                    )}
                  </NavLink>
                </li>
              ))}
            </ul>
          </div>
        ))}
      </nav>

      {/* Collapse toggle for mobile */}
      {collapsed && (
        <button
          onClick={onCollapse}
          className="flex items-center justify-center h-10 border-t border-slate-200 dark:border-slate-800 text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 transition-colors"
        >
          <ChevronRight className="w-4 h-4 rotate-180" />
        </button>
      )}
    </aside>
  )
}
