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

export async function login(data: LoginRequest): Promise<LoginResponse> {
  const res = await apiClient.post<LoginResponse>('/api/v1/auth/login', data)
  return res.data
}

export async function getMe(): Promise<UserInfo> {
  const res = await apiClient.get<UserInfo>('/api/v1/auth/me')
  return res.data
}

export async function changePassword(currentPassword: string, newPassword: string): Promise<void> {
  await apiClient.post('/api/v1/auth/change-password', {
    current_password: currentPassword,
    new_password: newPassword,
  })
}
