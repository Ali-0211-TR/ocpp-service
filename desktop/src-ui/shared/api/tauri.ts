import { invoke } from '@tauri-apps/api/core'

// ── Types ──────────────────────────────────────────────────────────

export interface ServerInfo {
  running: boolean
  api_port: number
  ws_port: number
  config_path: string
}

export interface AppConfig {
  server: {
    api_host: string
    api_port: number
    ws_host: string
    ws_port: number
    shutdown_timeout: number
  }
  database: {
    path: string
  }
  security: {
    jwt_secret: string
    jwt_expiration_hours: number
  }
  logging: {
    level: string
    format: string
  }
  admin: {
    username: string
    email: string
    password: string
  }
  [key: string]: unknown
}

// ── Tauri IPC Commands ─────────────────────────────────────────────

/** Get current server status info. */
export async function getServerStatus(): Promise<ServerInfo> {
  return invoke<ServerInfo>('server_status')
}

/** Start the OCPP server. */
export async function startServer(): Promise<void> {
  return invoke('server_start')
}

/** Stop the OCPP server. */
export async function stopServer(): Promise<void> {
  return invoke('server_stop')
}

/** Restart the OCPP server (stop → reload config → start). */
export async function restartServer(): Promise<void> {
  return invoke('server_restart')
}

/** Get the server configuration. */
export async function getConfig(): Promise<AppConfig> {
  return invoke<AppConfig>('get_config')
}

/** Save configuration to disk. */
export async function saveConfig(config: AppConfig): Promise<void> {
  return invoke('save_config', { config })
}

/** Save config and restart server. */
export async function saveAndRestart(config: AppConfig): Promise<void> {
  return invoke('save_and_restart', { config })
}

/** Get config file path. */
export async function getConfigPath(): Promise<string> {
  return invoke<string>('get_config_path')
}
