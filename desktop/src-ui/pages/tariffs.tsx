import { Card, CardContent } from '@/shared/ui/shadcn/card'
import { Tags } from 'lucide-react'

export function TariffsPage() {
  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold tracking-tight">Тарифы</h2>
        <p className="text-muted-foreground">Управление тарифными планами</p>
      </div>
      <Card>
        <CardContent className="flex flex-col items-center justify-center py-16 text-center">
          <Tags className="mb-4 h-12 w-12 text-muted-foreground/50" />
          <h3 className="text-lg font-semibold">Тарифы</h3>
          <p className="mt-1 max-w-md text-sm text-muted-foreground">
            Создание и управление тарифами для зарядных станций: за энергию (кВт·ч),
            за время, фиксированная стоимость. Привязка тарифов к станциям.
          </p>
        </CardContent>
      </Card>
    </div>
  )
}
