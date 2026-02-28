import { Card, CardContent } from '@/shared/ui/shadcn/card'
import { Activity } from 'lucide-react'

export function MonitoringPage() {
  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold tracking-tight">Мониторинг</h2>
        <p className="text-muted-foreground">Heartbeat и состояние подключений</p>
      </div>
      <Card>
        <CardContent className="flex flex-col items-center justify-center py-16 text-center">
          <Activity className="mb-4 h-12 w-12 text-muted-foreground/50" />
          <h3 className="text-lg font-semibold">Мониторинг</h3>
          <p className="mt-1 max-w-md text-sm text-muted-foreground">
            Отслеживание heartbeat-ов станций, статусов подключений, метрик
            Prometheus. Уведомления при потере связи со станцией.
          </p>
        </CardContent>
      </Card>
    </div>
  )
}
