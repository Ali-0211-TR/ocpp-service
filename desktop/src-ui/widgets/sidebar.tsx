import { NavLink } from 'react-router-dom'
import { cn } from '@/shared/lib/utils'
import { useAuthStore, useCurrentUser } from '@/shared/store/auth-store'
import {
  LayoutDashboard,
  Plug,
  Receipt,
  BarChart3,
  Tags,
  CreditCard,
  Activity,
  KeyRound,
  Settings,
  Zap,
  LogOut,
  User,
} from 'lucide-react'

const navItems = [
  { to: '/', icon: LayoutDashboard, label: 'Панель' },
  { to: '/charge-points', icon: Plug, label: 'Станции' },
  { to: '/transactions', icon: Receipt, label: 'Транзакции' },
  { to: '/analytics', icon: BarChart3, label: 'Аналитика' },
  { to: '/tariffs', icon: Tags, label: 'Тарифы' },
  { to: '/id-tags', icon: CreditCard, label: 'ID Tags' },
  { to: '/monitoring', icon: Activity, label: 'Мониторинг' },
  { to: '/api-keys', icon: KeyRound, label: 'API Ключи' },
  { to: '/settings', icon: Settings, label: 'Настройки' },
]

export function Sidebar() {
  const user = useCurrentUser()
  const clearAuth = useAuthStore((s) => s.clearAuth)

  return (
    <aside className="flex h-screen w-56 flex-col border-r bg-card">
      {/* Logo */}
      <div className="flex items-center gap-2 border-b px-4 py-4">
        <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-primary">
          <Zap className="h-5 w-5 text-primary-foreground" />
        </div>
        <div>
          <h1 className="text-sm font-bold leading-tight">Texnouz CSMS</h1>
          <p className="text-[10px] text-muted-foreground">Central System</p>
        </div>
      </div>

      {/* Navigation */}
      <nav className="flex-1 space-y-1 p-2">
        {navItems.map(({ to, icon: Icon, label }) => (
          <NavLink
            key={to}
            to={to}
            end={to === '/'}
            className={({ isActive }) =>
              cn(
                'flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors',
                isActive
                  ? 'bg-primary/10 text-primary'
                  : 'text-muted-foreground hover:bg-accent hover:text-accent-foreground'
              )
            }
          >
            <Icon className="h-4 w-4" />
            {label}
          </NavLink>
        ))}
      </nav>

      {/* User & Logout */}
      <div className="border-t p-2">
        {user && (
          <div className="flex items-center justify-between rounded-md px-3 py-2">
            <div className="flex items-center gap-2 min-w-0">
              <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-primary/10">
                <User className="h-3.5 w-3.5 text-primary" />
              </div>
              <div className="min-w-0">
                <p className="truncate text-xs font-medium">{user.username}</p>
                <p className="truncate text-[10px] text-muted-foreground">{user.role}</p>
              </div>
            </div>
            <button
              onClick={clearAuth}
              className="shrink-0 rounded-md p-1.5 text-muted-foreground hover:bg-accent hover:text-accent-foreground transition-colors"
              title="Выйти"
            >
              <LogOut className="h-3.5 w-3.5" />
            </button>
          </div>
        )}
        <p className="mt-1 text-[10px] text-muted-foreground text-center">v0.1.0 • OCPP 1.6 / 2.0.1</p>
      </div>
    </aside>
  )
}
