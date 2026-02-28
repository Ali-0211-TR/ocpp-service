import { useEffect, useState } from 'react'
import { Routes, Route } from 'react-router-dom'
import { Sidebar } from '@/widgets/sidebar'
import { ServerStatusBar } from '@/widgets/server-status-bar'
import { DashboardPage } from '@/pages/dashboard'
import { ChargePointsPage } from '@/pages/charge-points'
import { TransactionsPage } from '@/pages/transactions'
import { AnalyticsPage } from '@/pages/analytics'
import { TariffsPage } from '@/pages/tariffs'
import { IdTagsPage } from '@/pages/id-tags'
import { MonitoringPage } from '@/pages/monitoring'
import { ApiKeysPage } from '@/pages/api-keys'
import { SettingsPage } from '@/pages/settings'
import { LoginPage } from '@/pages/login'
import { useAuthStore, useIsAuthenticated } from '@/shared/store/auth-store'
import { getMe } from '@/shared/api/auth'
import { getServerStatus } from '@/shared/api/tauri'
import { updateApiBaseUrl } from '@/shared/api/client'
import { Loader2 } from 'lucide-react'

export function App() {
  const isAuthenticated = useIsAuthenticated()
  const [checking, setChecking] = useState(true)
  const { setAuth, clearAuth } = useAuthStore()

  // On mount, validate the stored token
  useEffect(() => {
    const validate = async () => {
      const stored = useAuthStore.getState()
      if (!stored.token) {
        setChecking(false)
        return
      }

      try {
        const info = await getServerStatus()
        if (!info.running) {
          // Server not ready yet — keep token, wait
          setChecking(false)
          return
        }
        updateApiBaseUrl(info.api_port)
        const user = await getMe()
        setAuth(stored.token, user)
      } catch {
        clearAuth()
      } finally {
        setChecking(false)
      }
    }
    validate()
  }, [])

  if (checking) {
    return (
      <div className="flex h-screen items-center justify-center bg-background">
        <div className="flex flex-col items-center gap-3">
          <Loader2 className="h-8 w-8 animate-spin text-primary" />
          <p className="text-sm text-muted-foreground">Проверка авторизации...</p>
        </div>
      </div>
    )
  }

  if (!isAuthenticated) {
    return <LoginPage />
  }

  return (
    <div className="flex h-screen overflow-hidden">
      <Sidebar />
      <div className="flex flex-1 flex-col overflow-hidden">
        <ServerStatusBar />
        <main className="flex-1 overflow-y-auto p-6">
          <Routes>
            <Route path="/" element={<DashboardPage />} />
            <Route path="/charge-points" element={<ChargePointsPage />} />
            <Route path="/transactions" element={<TransactionsPage />} />
            <Route path="/analytics" element={<AnalyticsPage />} />
            <Route path="/tariffs" element={<TariffsPage />} />
            <Route path="/id-tags" element={<IdTagsPage />} />
            <Route path="/monitoring" element={<MonitoringPage />} />
            <Route path="/api-keys" element={<ApiKeysPage />} />
            <Route path="/settings" element={<SettingsPage />} />
          </Routes>
        </main>
      </div>
    </div>
  )
}
