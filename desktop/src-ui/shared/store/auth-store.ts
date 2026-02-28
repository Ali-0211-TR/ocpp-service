import { create } from 'zustand'
import { persist } from 'zustand/middleware'
import type { UserInfo } from '@/shared/api/auth'

interface AuthState {
  token: string | null
  user: UserInfo | null
  isAuthenticated: boolean
  setAuth: (token: string, user: UserInfo) => void
  clearAuth: () => void
  setUser: (user: UserInfo) => void
}

export const useAuthStore = create<AuthState>()(
  persist(
    (set) => ({
      token: null,
      user: null,
      isAuthenticated: false,

      setAuth: (token, user) => {
        set({ token, user, isAuthenticated: true })
      },

      clearAuth: () => {
        set({ token: null, user: null, isAuthenticated: false })
      },

      setUser: (user) => {
        set({ user })
      },
    }),
    {
      name: 'csms-auth',
      partialize: (state) => ({
        token: state.token,
        user: state.user,
        isAuthenticated: state.isAuthenticated,
      }),
    }
  )
)

// ── Selectors ──────────────────────────────────────────────────────

export const useIsAuthenticated = () => useAuthStore((s) => s.isAuthenticated)
export const useCurrentUser = () => useAuthStore((s) => s.user)
export const useAuthToken = () => useAuthStore((s) => s.token)
