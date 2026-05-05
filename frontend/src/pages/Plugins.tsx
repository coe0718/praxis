import { useParams, Link } from 'react-router-dom'
import { useQuery } from '@tanstack/react-query'
import { Package, ArrowLeft } from 'lucide-react'
import { fetchPluginTabs } from '../lib/api'
import { PluginTabRenderer } from '../components/layout/PluginTabRenderer'
import { PageSpinner } from '../components/ui/Spinner'
import { Empty } from '../components/ui/Empty'
import { Card } from '../components/ui/Card'

export function Plugins() {
  const { tabId } = useParams<{ tabId?: string }>()
  const { data, isLoading, error } = useQuery({
    queryKey: ['plugin-tabs'],
    queryFn: fetchPluginTabs,
    staleTime: 60_000,
  })

  const tabs = data?.tabs ?? []
  const activeTab = tabs.find((t) => t.id === tabId)

  if (isLoading) return <PageSpinner />

  if (error) {
    return (
      <div className="p-6 text-red-500">
        Failed to load plugins: {(error as Error).message}
      </div>
    )
  }

  if (tabs.length === 0) {
    return (
      <div className="space-y-6">
        <div className="page-header">
          <h1 className="page-title">Plugins</h1>
          <p className="page-subtitle">Dashboard plugin tabs</p>
        </div>
        <Card>
          <Empty
            icon={<Package className="w-8 h-8" />}
            title="No plugins installed"
            description="Install plugins to add custom dashboard tabs. Plugins define tabs in their plugin.toml under [dashboard]."
          />
        </Card>
      </div>
    )
  }

  // Tab selector — show all available tabs
  if (!tabId) {
    return (
      <div className="space-y-6">
        <div className="page-header">
          <h1 className="page-title">Plugins</h1>
          <p className="page-subtitle">{tabs.length} tab{tabs.length !== 1 ? 's' : ''} available</p>
        </div>
        <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
          {tabs.map((tab) => (
            <Link key={tab.id} to={`/plugins/${tab.id}`}>
              <Card className="h-full cursor-pointer hover:shadow-md transition-shadow">
                <div className="flex items-center gap-3">
                  <div className="w-10 h-10 rounded-lg bg-brand-50 dark:bg-brand-900/20 flex items-center justify-center">
                    <Package className="w-5 h-5 text-brand-600 dark:text-brand-400" />
                  </div>
                  <div>
                    <h3 className="font-medium text-slate-900 dark:text-slate-100">{tab.label}</h3>
                    <p className="text-xs text-slate-500">Click to open</p>
                  </div>
                </div>
              </Card>
            </Link>
          ))}
        </div>
      </div>
    )
  }

  // Active tab view
  return (
    <div className="space-y-6 h-full flex flex-col">
      <div className="page-header">
        <div className="flex items-center gap-3">
          <Link
            to="/plugins"
            className="p-1.5 rounded-lg hover:bg-slate-100 dark:hover:bg-slate-800 text-slate-400 hover:text-slate-600 dark:hover:text-slate-300 transition-colors"
          >
            <ArrowLeft className="w-4 h-4" />
          </Link>
          <div>
            <h1 className="page-title">{activeTab?.label ?? tabId}</h1>
            <p className="page-subtitle">Plugin tab</p>
          </div>
        </div>
      </div>
      <div className="flex-1 min-h-0">
        {activeTab ? (
          <PluginTabRenderer contentUrl={activeTab.content_url} />
        ) : (
          <div className="p-6 text-slate-500">Tab not found</div>
        )}
      </div>
    </div>
  )
}