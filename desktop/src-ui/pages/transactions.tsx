import { Card, CardContent } from '@/shared/ui/shadcn/card'
import { Receipt } from 'lucide-react'

export function TransactionsPage() {
  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold tracking-tight">Транзакции</h2>
        <p className="text-muted-foreground">История зарядных сессий</p>
      </div>
      <Card>
        <CardContent className="flex flex-col items-center justify-center py-16 text-center">
          <Receipt className="mb-4 h-12 w-12 text-muted-foreground/50" />
          <h3 className="text-lg font-semibold">Транзакции</h3>
          <p className="mt-1 max-w-md text-sm text-muted-foreground">
            Здесь будет отображаться история зарядных сессий с фильтрацией по
            станции, дате, статусу и пользователю. Включает экспорт в CSV.
          </p>
        </CardContent>
      </Card>
    </div>
  )
}
