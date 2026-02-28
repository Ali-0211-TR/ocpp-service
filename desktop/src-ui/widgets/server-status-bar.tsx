import { useEffect, useState } from 'react'
import { Badge } from '@/shared/ui/shadcn/badge'
import { Button } from '@/shared/ui/shadcn/button'
import { getServerStatus, type ServerInfo } from '@/shared/api/tauri'
import { Play, Square, RefreshCw } from 'lucide-react'
import * as tauri from '@/shared/api/tauri'

export function ServerStatusBar() {
  const [info, setInfo] = useState<ServerInfo | null>(null)

  const refresh = async () => {
    try {
      setInfo(await getServerStatus())
    } catch {
      setInfo(null)
    }
  }

  useEffect(() => {
    refresh()
    const id = setInterval(refresh, 3000)
    return () => clearInterval(id)
  }, [])

  const running = info?.running ?? false

  return (
    <header className="flex items-center justify-between border-b bg-card px-4 py-2">
      <div className="flex items-center gap-3">
        <Badge variant={running ? 'success' : 'destructive'}>
          <span className="mr-1.5 inline-block h-2 w-2 rounded-full bg-current" />
          {running ? 'Работает' : 'Остановлен'}
        </Badge>
        {info && running && (
          <span className="text-xs text-muted-foreground">
            API :{info.api_port} • WS :{info.ws_port}
          </span>
        )}
      </div>

      <div className="flex items-center gap-1">
        {!running && (
          <Button
            variant="ghost"
            size="sm"
            onClick={async () => { await tauri.startServer(); refresh() }}
          >
            <Play className="mr-1 h-3.5 w-3.5" />
            Запустить
          </Button>
        )}
        {running && (
          <>
            <Button
              variant="ghost"
              size="sm"
              onClick={async () => { await tauri.restartServer(); refresh() }}
            >
              <RefreshCw className="mr-1 h-3.5 w-3.5" />
              Рестарт
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={async () => { await tauri.stopServer(); refresh() }}
            >
              <Square className="mr-1 h-3.5 w-3.5" />
              Стоп
            </Button>
          </>
        )}
      </div>
    </header>
  )
}
