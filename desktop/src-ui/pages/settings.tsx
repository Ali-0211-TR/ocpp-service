import { useEffect, useState } from 'react'
import { Card, CardContent, CardHeader, CardTitle, CardDescription, CardFooter } from '@/shared/ui/shadcn/card'
import { Button } from '@/shared/ui/shadcn/button'
import { Badge } from '@/shared/ui/shadcn/badge'
import { getServerStatus } from '@/shared/api/tauri'
import * as tauri from '@/shared/api/tauri'
import type { AppConfig } from '@/shared/api/tauri'
import { Settings as SettingsIcon, Save, RefreshCw, FolderOpen } from 'lucide-react'
import { toast } from 'sonner'

export function SettingsPage() {
  const [config, setConfig] = useState<AppConfig | null>(null)
  const [configPath, setConfigPath] = useState('')
  const [saving, setSaving] = useState(false)

  useEffect(() => {
    const load = async () => {
      try {
        setConfig(await tauri.getConfig())
        setConfigPath(await tauri.getConfigPath())
      } catch (e) {
        console.error('Failed to load config:', e)
      }
    }
    load()
  }, [])

  const handleSaveAndRestart = async () => {
    if (!config) return
    setSaving(true)
    try {
      await tauri.saveAndRestart(config)
      toast.success('Настройки сохранены, сервер перезапущен')
    } catch (e) {
      toast.error(`Ошибка: ${e}`)
    } finally {
      setSaving(false)
    }
  }

  if (!config) {
    return (
      <div className="flex items-center justify-center py-16">
        <p className="text-muted-foreground">Загрузка настроек...</p>
      </div>
    )
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold tracking-tight">Настройки</h2>
          <p className="text-muted-foreground">Конфигурация OCPP Central System</p>
        </div>
        <div className="flex items-center gap-2">
          <Badge variant="outline" className="text-xs">
            <FolderOpen className="mr-1 h-3 w-3" />
            {configPath}
          </Badge>
        </div>
      </div>

      {/* Server Settings */}
      <Card>
        <CardHeader>
          <CardTitle className="text-base">Сервер</CardTitle>
          <CardDescription>Порты и хост для REST API и WebSocket</CardDescription>
        </CardHeader>
        <CardContent className="grid gap-4 sm:grid-cols-2">
          <SettingsField
            label="API Host"
            value={config.server.api_host}
            onChange={(v) => setConfig({ ...config, server: { ...config.server, api_host: v } })}
          />
          <SettingsField
            label="API Port"
            value={String(config.server.api_port)}
            type="number"
            onChange={(v) => setConfig({ ...config, server: { ...config.server, api_port: Number(v) } })}
          />
          <SettingsField
            label="WS Host"
            value={config.server.ws_host}
            onChange={(v) => setConfig({ ...config, server: { ...config.server, ws_host: v } })}
          />
          <SettingsField
            label="WS Port"
            value={String(config.server.ws_port)}
            type="number"
            onChange={(v) => setConfig({ ...config, server: { ...config.server, ws_port: Number(v) } })}
          />
        </CardContent>
      </Card>

      {/* Logging Settings */}
      <Card>
        <CardHeader>
          <CardTitle className="text-base">Логирование</CardTitle>
          <CardDescription>Уровень и формат логов</CardDescription>
        </CardHeader>
        <CardContent className="grid gap-4 sm:grid-cols-2">
          <SettingsField
            label="Уровень"
            value={config.logging.level}
            onChange={(v) => setConfig({ ...config, logging: { ...config.logging, level: v } })}
          />
          <SettingsField
            label="Формат"
            value={config.logging.format}
            onChange={(v) => setConfig({ ...config, logging: { ...config.logging, format: v } })}
          />
        </CardContent>
      </Card>

      {/* Security Settings */}
      <Card>
        <CardHeader>
          <CardTitle className="text-base">Безопасность</CardTitle>
          <CardDescription>JWT-токены и срок действия</CardDescription>
        </CardHeader>
        <CardContent className="grid gap-4 sm:grid-cols-2">
          <SettingsField
            label="JWT Secret"
            value={config.security.jwt_secret}
            type="password"
            onChange={(v) => setConfig({ ...config, security: { ...config.security, jwt_secret: v } })}
          />
          <SettingsField
            label="JWT Expiration (часы)"
            value={String(config.security.jwt_expiration_hours)}
            type="number"
            onChange={(v) => setConfig({ ...config, security: { ...config.security, jwt_expiration_hours: Number(v) } })}
          />
        </CardContent>
      </Card>

      {/* Actions */}
      <div className="flex justify-end gap-3">
        <Button
          variant="outline"
          onClick={async () => {
            setConfig(await tauri.getConfig())
            toast.info('Настройки перезагружены из файла')
          }}
        >
          <RefreshCw className="mr-2 h-4 w-4" />
          Сбросить
        </Button>
        <Button onClick={handleSaveAndRestart} disabled={saving}>
          <Save className="mr-2 h-4 w-4" />
          {saving ? 'Сохранение...' : 'Сохранить и перезапустить'}
        </Button>
      </div>
    </div>
  )
}

// ── Settings Field ─────────────────────────────────────────────────

function SettingsField({
  label,
  value,
  type = 'text',
  onChange,
}: {
  label: string
  value: string
  type?: string
  onChange: (value: string) => void
}) {
  return (
    <div className="space-y-1.5">
      <label className="text-sm font-medium">{label}</label>
      <input
        type={type}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="flex h-9 w-full rounded-md border border-input bg-background px-3 py-1 text-sm shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
      />
    </div>
  )
}
