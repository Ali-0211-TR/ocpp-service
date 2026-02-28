import { useEffect, useState } from 'react'
import { Card, CardContent, CardHeader, CardTitle } from '@/shared/ui/shadcn/card'
import { Badge } from '@/shared/ui/shadcn/badge'
import { getServerStatus, type ServerInfo } from '@/shared/api/tauri'
import { apiClient, updateApiBaseUrl } from '@/shared/api/client'
import {
  Plug,
  PlugZap,
  Receipt,
  Zap,
  Activity,
  Server,
  Wifi,
  WifiOff,
} from 'lucide-react'

interface Stats {
  total: number
  online: number
  offline: number
  charging: number
}

export function DashboardPage() {
  const [serverInfo, setServerInfo] = useState<ServerInfo | null>(null)
  const [stats, setStats] = useState<Stats | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    const init = async () => {
      try {
        const info = await getServerStatus()
        setServerInfo(info)
        if (info.running) {
          updateApiBaseUrl(info.api_port)
          try {
            const res = await apiClient.get('/api/v1/charge-points/stats')
            // Backend wraps response in ApiResponse: { success, data }
            setStats(res.data?.data ?? res.data)
            setError(null)
          } catch {
            setError('Не удалось загрузить статистику')
          }
        }
      } catch {
        setError('Нет связи с сервером')
      }
    }
    init()
    const id = setInterval(init, 10_000)
    return () => clearInterval(id)
  }, [])

  const running = serverInfo?.running ?? false

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold tracking-tight">Панель управления</h2>
        <p className="text-muted-foreground">
          Обзор состояния OCPP Central System
        </p>
      </div>

      {/* Server Status Card */}
      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="flex items-center gap-2 text-base">
            <Server className="h-4 w-4" />
            Сервер
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
            <div className="flex items-center gap-3">
              <div className={`h-3 w-3 rounded-full ${running ? 'bg-emerald-500 animate-pulse' : 'bg-red-500'}`} />
              <div>
                <p className="text-sm font-medium">Статус</p>
                <p className="text-xs text-muted-foreground">
                  {running ? 'Работает' : 'Остановлен'}
                </p>
              </div>
            </div>
            <div>
              <p className="text-sm font-medium">REST API</p>
              <p className="text-xs text-muted-foreground">
                {running ? `http://localhost:${serverInfo?.api_port}` : '—'}
              </p>
            </div>
            <div>
              <p className="text-sm font-medium">WebSocket</p>
              <p className="text-xs text-muted-foreground">
                {running ? `ws://localhost:${serverInfo?.ws_port}` : '—'}
              </p>
            </div>
            <div>
              <p className="text-sm font-medium">Конфигурация</p>
              <p className="text-xs text-muted-foreground truncate" title={serverInfo?.config_path}>
                {serverInfo?.config_path ?? '—'}
              </p>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Stats Grid */}
      {running && (
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
          <StatCard
            title="Зарядные станции"
            value={stats?.total ?? 0}
            description="Зарегистрировано"
            icon={Plug}
          />
          <StatCard
            title="Онлайн"
            value={stats?.online ?? 0}
            description={`из ${stats?.total ?? 0} станций`}
            icon={stats?.online ? Wifi : WifiOff}
            variant={stats?.online ? 'success' : 'warning'}
          />
          <StatCard
            title="Оффлайн"
            value={stats?.offline ?? 0}
            description="Нет связи"
            icon={WifiOff}
            variant={stats?.offline ? 'warning' : 'default'}
          />
          <StatCard
            title="Заряжаются"
            value={stats?.charging ?? 0}
            description="Активные сессии"
            icon={PlugZap}
            variant={stats?.charging ? 'success' : 'default'}
          />
        </div>
      )}

      {!running && (
        <Card>
          <CardContent className="flex flex-col items-center justify-center py-12 text-center">
            <WifiOff className="mb-4 h-12 w-12 text-muted-foreground/50" />
            <h3 className="text-lg font-semibold">Сервер остановлен</h3>
            <p className="mt-1 text-sm text-muted-foreground">
              Запустите сервер через панель управления или системный трей
            </p>
          </CardContent>
        </Card>
      )}

      {error && (
        <Card className="border-destructive/50">
          <CardContent className="py-4">
            <p className="text-sm text-destructive">{error}</p>
          </CardContent>
        </Card>
      )}
    </div>
  )
}

// ── Stat Card ──────────────────────────────────────────────────────

function StatCard({
  title,
  value,
  description,
  icon: Icon,
  variant = 'default',
}: {
  title: string
  value: number | string
  description: string
  icon: React.ComponentType<{ className?: string }>
  variant?: 'default' | 'success' | 'warning'
}) {
  return (
    <Card>
      <CardContent className="p-6">
        <div className="flex items-center justify-between">
          <div>
            <p className="text-sm font-medium text-muted-foreground">{title}</p>
            <p className="mt-1 text-3xl font-bold">{value}</p>
            <p className="mt-1 text-xs text-muted-foreground">{description}</p>
          </div>
          <div
            className={`flex h-12 w-12 items-center justify-center rounded-lg ${
              variant === 'success'
                ? 'bg-emerald-500/10 text-emerald-500'
                : variant === 'warning'
                  ? 'bg-amber-500/10 text-amber-500'
                  : 'bg-primary/10 text-primary'
            }`}
          >
            <Icon className="h-6 w-6" />
          </div>
        </div>
      </CardContent>
    </Card>
  )
}
