import axios from 'axios'
import { getServerStatus } from './tauri'

/** Create an Axios instance that targets the embedded CSMS REST API. */
async function getApiPort(): Promise<number> {
  try {
    const info = await getServerStatus()
    return info.api_port
  } catch {
    return 3000 // fallback
  }
}

let _apiPort: number | null = null

export async function initApiClient(): Promise<void> {
  _apiPort = (await getApiPort()) || 3000
}

/** Get the base URL for the REST API. */
export function getApiBaseUrl(): string {
  return `http://localhost:${_apiPort ?? 3000}`
}

/** Axios client for the embedded CSMS REST API. */
export const apiClient = axios.create({
  baseURL: `http://localhost:3000`,
  timeout: 15_000,
  headers: {
    'Content-Type': 'application/json',
  },
})

/** Update the API client base URL after getting the actual port. */
export function updateApiBaseUrl(port: number) {
  _apiPort = port
  apiClient.defaults.baseURL = `http://localhost:${port}`
}

// ── Auth interceptor ──────────────────────────────────────────────

/** Get auth token from Zustand persisted store (localStorage). */
export function getAuthToken(): string | null {
  try {
    const raw = localStorage.getItem('csms-auth')
    if (raw) {
      const parsed = JSON.parse(raw)
      return parsed?.state?.token ?? null
    }
  } catch { /* ignore */ }
  return null
}

/** @deprecated Use useAuthStore().setAuth() instead */
export function setAuthToken(_token: string | null) {
  // no-op — managed by Zustand store now
}

apiClient.interceptors.request.use((config) => {
  const token = getAuthToken()
  if (token) {
    config.headers.Authorization = `Bearer ${token}`
  }
  return config
})

/** Auto-logout on 401 responses (expired / invalid token). */
apiClient.interceptors.response.use(
  (response) => response,
  (error) => {
    if (error?.response?.status === 401) {
      // Clear persisted auth state
      try {
        localStorage.removeItem('csms-auth')
      } catch { /* ignore */ }
      // Force reload to show login
      window.location.reload()
    }
    return Promise.reject(error)
  }
)
