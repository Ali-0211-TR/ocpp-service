import { Card, CardContent } from '@/shared/ui/shadcn/card'
import { KeyRound } from 'lucide-react'

export function ApiKeysPage() {
  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold tracking-tight">API Ключи</h2>
        <p className="text-muted-foreground">Управление ключами для внешних интеграций</p>
      </div>
      <Card>
        <CardContent className="flex flex-col items-center justify-center py-16 text-center">
          <KeyRound className="mb-4 h-12 w-12 text-muted-foreground/50" />
          <h3 className="text-lg font-semibold">API Ключи</h3>
          <p className="mt-1 max-w-md text-sm text-muted-foreground">
            Создание и отзыв API-ключей для доступа к REST API.
            Поддержка ролевого доступа и срока действия.
          </p>
        </CardContent>
      </Card>
    </div>
  )
}
