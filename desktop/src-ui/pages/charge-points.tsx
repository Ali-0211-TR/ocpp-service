import { useEffect, useState } from 'react'
import { Card, CardContent, CardHeader, CardTitle } from '@/shared/ui/shadcn/card'
import { Badge } from '@/shared/ui/shadcn/badge'
import { Button } from '@/shared/ui/shadcn/button'
import { apiClient, updateApiBaseUrl } from '@/shared/api/client'
import { getServerStatus } from '@/shared/api/tauri'
import { Plug, PlugZap, Wifi, WifiOff, MoreVertical } from 'lucide-react'

interface ChargePoint {
  id: string
  vendor: string
  model: string
  serial_number: string | null
  status: string
  last_heartbeat: string | null
  ocpp_version: string
  connectors: Connector[]
}

interface Connector {
  id: number
  connector_id: number
  status: string
  error_code: string | null
}

export function ChargePointsPage() {
  const [chargePoints, setChargePoints] = useState<ChargePoint[]>([])
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    const load = async () => {
      try {
        const info = await getServerStatus()
        if (!info.running) return
        updateApiBaseUrl(info.api_port)
        const res = await apiClient.get('/api/v1/charge-points')
        // Backend wraps response in ApiResponse: { success, data }
        const list = res.data?.data ?? res.data
        setChargePoints(Array.isArray(list) ? list : [])
      } catch (e) {
        console.error('Failed to load charge points:', e)
      } finally {
        setLoading(false)
      }
    }
    load()
  }, [])

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold tracking-tight">Зарядные станции</h2>
          <p className="text-muted-foreground">
            Управление подключёнными зарядными станциями
          </p>
        </div>
        <Badge variant="outline">{chargePoints.length} станций</Badge>
      </div>

      {loading ? (
        <div className="flex items-center justify-center py-12">
          <p className="text-sm text-muted-foreground">Загрузка...</p>
        </div>
      ) : chargePoints.length === 0 ? (
        <Card>
          <CardContent className="flex flex-col items-center justify-center py-12 text-center">
            <Plug className="mb-4 h-12 w-12 text-muted-foreground/50" />
            <h3 className="text-lg font-semibold">Нет станций</h3>
            <p className="mt-1 text-sm text-muted-foreground">
              Зарядные станции появятся здесь после подключения по OCPP
            </p>
          </CardContent>
        </Card>
      ) : (
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {chargePoints.map((cp) => (
            <ChargePointCard key={cp.id} chargePoint={cp} />
          ))}
        </div>
      )}
    </div>
  )
}

function ChargePointCard({ chargePoint }: { chargePoint: ChargePoint }) {
  const isOnline = chargePoint.status === 'Available' || chargePoint.status === 'Charging'
  const connectorCount = chargePoint.connectors?.length ?? 0

  return (
    <Card className="transition-shadow hover:shadow-md">
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <CardTitle className="text-base">{chargePoint.id}</CardTitle>
          <Badge variant={isOnline ? 'success' : 'destructive'} className="text-[10px]">
            {isOnline ? 'Online' : 'Offline'}
          </Badge>
        </div>
      </CardHeader>
      <CardContent className="space-y-3">
        <div className="grid grid-cols-2 gap-2 text-xs text-muted-foreground">
          <div>
            <span className="font-medium text-foreground">Производитель</span>
            <p>{chargePoint.vendor}</p>
          </div>
          <div>
            <span className="font-medium text-foreground">Модель</span>
            <p>{chargePoint.model}</p>
          </div>
        </div>

        <div className="flex items-center justify-between text-xs">
          <div className="flex items-center gap-1 text-muted-foreground">
            <PlugZap className="h-3.5 w-3.5" />
            {connectorCount} коннектор(ов)
          </div>
          <Badge variant="outline" className="text-[10px]">
            OCPP {chargePoint.ocpp_version}
          </Badge>
        </div>
      </CardContent>
    </Card>
  )
}
