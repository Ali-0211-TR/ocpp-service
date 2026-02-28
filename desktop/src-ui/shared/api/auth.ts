import { apiClient } from './client'

// ── Types ──────────────────────────────────────────────────────────

export interface LoginRequest {
  username: string
  password: string
}

export interface UserInfo {
  id: string
  username: string
  email: string
  role: string
}

export interface LoginResponse {
  token: string
  token_type: string
  expires_in: number
  user: UserInfo
}

// ── API ────────────────────────────────────────────────────────────

/**
 * Backend wraps ALL responses in ApiResponse: { success, data, error? }
 * We must unwrap the inner `data` field to get the actual payload.
 */

export async function login(data: LoginRequest): Promise<LoginResponse> {
  const res = await apiClient.post('/api/v1/auth/login', data)
  // Unwrap ApiResponse: { success: true, data: LoginResponse }
  return res.data?.data ?? res.data
}

export async function getMe(): Promise<UserInfo> {
  const res = await apiClient.get('/api/v1/auth/me')
  // Unwrap ApiResponse: { success: true, data: UserInfo }
  return res.data?.data ?? res.data
}

export async function changePassword(currentPassword: string, newPassword: string): Promise<void> {
  await apiClient.post('/api/v1/auth/change-password', {
    current_password: currentPassword,
    new_password: newPassword,
  })
}
