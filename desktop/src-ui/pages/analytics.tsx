import { Card, CardContent } from '@/shared/ui/shadcn/card'
import { BarChart3 } from 'lucide-react'

export function AnalyticsPage() {
  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold tracking-tight">Аналитика</h2>
        <p className="text-muted-foreground">Статистика и графики работы CSMS</p>
      </div>
      <Card>
        <CardContent className="flex flex-col items-center justify-center py-16 text-center">
          <BarChart3 className="mb-4 h-12 w-12 text-muted-foreground/50" />
          <h3 className="text-lg font-semibold">Аналитика</h3>
          <p className="mt-1 max-w-md text-sm text-muted-foreground">
            Графики выручки, потребления энергии, пиковых часов и uptime станций.
            Данные обновляются в реальном времени.
          </p>
        </CardContent>
      </Card>
    </div>
  )
}
