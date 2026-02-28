import { Card, CardContent } from '@/shared/ui/shadcn/card'
import { CreditCard } from 'lucide-react'

export function IdTagsPage() {
  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold tracking-tight">ID Tags</h2>
        <p className="text-muted-foreground">Авторизационные RFID-метки</p>
      </div>
      <Card>
        <CardContent className="flex flex-col items-center justify-center py-16 text-center">
          <CreditCard className="mb-4 h-12 w-12 text-muted-foreground/50" />
          <h3 className="text-lg font-semibold">ID Tags</h3>
          <p className="mt-1 max-w-md text-sm text-muted-foreground">
            Управление RFID-метками для авторизации зарядных сессий.
            Поддержка статусов: Accepted, Blocked, Expired.
            Синхронизация локальных списков на станции.
          </p>
        </CardContent>
      </Card>
    </div>
  )
}
