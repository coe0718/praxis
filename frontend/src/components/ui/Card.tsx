import { cn } from '../../lib/utils'

interface CardProps {
  children: React.ReactNode
  className?: string
  elevated?: boolean
  padding?: 'none' | 'sm' | 'md' | 'lg'
}

const paddings = {
  none: '',
  sm: 'p-4',
  md: 'p-5',
  lg: 'p-6',
}

export function Card({ children, className, elevated, padding = 'md' }: CardProps) {
  return (
    <div className={cn(elevated ? 'card-elevated' : 'card', paddings[padding], className)}>
      {children}
    </div>
  )
}

interface StatCardProps {
  label: string
  value: React.ReactNode
  icon?: React.ReactNode
  trend?: 'up' | 'down' | 'neutral'
  className?: string
}

export function StatCard({ label, value, icon, className }: StatCardProps) {
  return (
    <Card className={cn('flex items-center gap-4', className)}>
      {icon && (
        <div className="flex-shrink-0 w-10 h-10 rounded-lg bg-brand-100 dark:bg-brand-900/30 flex items-center justify-center text-brand-600 dark:text-brand-400">
          {icon}
        </div>
      )}
      <div>
        <p className="stat-label">{label}</p>
        <p className="stat-value">{value}</p>
      </div>
    </Card>
  )
}
