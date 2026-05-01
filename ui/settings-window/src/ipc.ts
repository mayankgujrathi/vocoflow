export type IpcMethod = 'GET' | 'POST' | 'PUT' | 'DELETE' | 'PATCH'

export type IpcRequest<TPayload> = {
  request_id?: string
  method: IpcMethod
  endpoint: string
  payload: TPayload
}

export type IpcError = {
  code?: string
  message?: string
  field?: string
  expected?: string
  received?: string
}

export type IpcResponse<TPayload> = {
  request_id?: string
  ok: boolean
  kind: string
  payload: TPayload
  error?: IpcError
}

import type {
  AppSettings,
  HistoryPage,
  HistorySettings,
  LoggingSettings,
  SettingsFlashPayload,
  TranscriptionSettings,
} from './types/settings'

export type SettingsResponse = { settings: AppSettings; logs_dir: string; flash?: SettingsFlashPayload }
export type AboutLogsDirResponse = { logs_dir: string }
export type AboutOpenExternalUrlResponse = { url: string }
export type HistoryResponse = { history: HistoryPage }
export type HistoryDeleteResponse = { deleted: boolean }

export type SettingsEventChanged = {
  event: 'settings.changed'
  version: number
  updated_at_ms: number
  payload: { settings: AppSettings }
}

export type SettingsEventFlash = {
  event: 'settings.flash'
  version: number
  updated_at_ms: number
  payload: { flash: SettingsFlashPayload }
}

export type SettingsEventEnvelope = SettingsEventChanged | SettingsEventFlash

const makeRequestId = (): string => {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return crypto.randomUUID()
  }
  return String(Date.now())
}

const normalizeError = (errorLike: unknown): Required<Pick<IpcError, 'code' | 'message'>> & IpcError => {
  if (typeof errorLike === 'string') {
    return { code: 'UNKNOWN', message: errorLike }
  }
  if (!errorLike || typeof errorLike !== 'object') {
    return { code: 'UNKNOWN', message: 'Unknown IPC error' }
  }

  const candidate = errorLike as IpcError
  return {
    code: candidate.code || 'UNKNOWN',
    message: candidate.message || 'IPC request failed',
    field: candidate.field,
    expected: candidate.expected,
    received:
      typeof candidate.received === 'string'
        ? candidate.received
        : candidate.received != null
          ? JSON.stringify(candidate.received)
          : undefined,
  }
}

const formatErrorForUi = (error: IpcError): string => {
  const lines = [`code: ${error.code || 'UNKNOWN'}`, `message: ${error.message || 'IPC request failed'}`]
  if (error.field) lines.push(`field: ${error.field}`)
  if (error.expected) lines.push(`expected: ${error.expected}`)
  if (error.received) lines.push(`received: ${error.received}`)
  return lines.join('\n')
}

export const sendIpc = async <TPayloadReq, TPayloadRes>(
  request: IpcRequest<TPayloadReq>,
): Promise<IpcResponse<TPayloadRes>> => {
  const request_id = request.request_id || makeRequestId()
  const body = JSON.stringify({ ...request, request_id })

  return await new Promise((resolve, reject) => {
    const xhr = new XMLHttpRequest()
    xhr.open('POST', '/ipc', true)
    xhr.setRequestHeader('content-type', 'application/json')
    xhr.timeout = 10000

    xhr.onload = () => {
      let parsed: IpcResponse<TPayloadRes> | null = null

      try {
        parsed = JSON.parse(xhr.responseText || '{}') as IpcResponse<TPayloadRes>
      } catch {
        reject(new Error(`IPC response was not valid JSON (status=${xhr.status}, request_id=${request_id})`))
        return
      }

      if (xhr.status >= 400 || !parsed?.ok) {
        const normalized = normalizeError(parsed?.error || parsed)
        reject(new Error(formatErrorForUi(normalized)))
        return
      }

      resolve(parsed)
    }

    xhr.onerror = () => reject(new Error(`IPC request failed (network error, request_id=${request_id})`))
    xhr.ontimeout = () => reject(new Error(`IPC request timed out for request_id=${request_id}`))
    xhr.send(body)
  })
}

export const getAllSettings = async (): Promise<SettingsResponse> => {
  const reply = await sendIpc<Record<string, never>, SettingsResponse>({ method: 'GET', endpoint: '/settings', payload: {} })
  return reply.payload
}

export const updateStartOnLogin = async (start_on_login: boolean): Promise<AppSettings> => {
  const reply = await sendIpc<{ start_on_login: boolean }, SettingsResponse>({
    method: 'POST',
    endpoint: '/settings/update/start_on_login',
    payload: { start_on_login },
  })
  return reply.payload.settings
}

export const updateHotkey = async (binding: string, chord_timeout_ms?: number): Promise<AppSettings> => {
  const reply = await sendIpc<{ binding: string; chord_timeout_ms?: number }, SettingsResponse>({
    method: 'POST',
    endpoint: '/settings/update/hotkey',
    payload: { binding, chord_timeout_ms },
  })
  return reply.payload.settings
}

export const updateLogging = async (logging: LoggingSettings): Promise<AppSettings> => {
  const reply = await sendIpc<{ logging: LoggingSettings }, SettingsResponse>({
    method: 'POST',
    endpoint: '/settings/update/logging',
    payload: { logging },
  })
  return reply.payload.settings
}

export const updateTranscription = async (transcription: TranscriptionSettings): Promise<AppSettings> => {
  const reply = await sendIpc<{ transcription: TranscriptionSettings }, SettingsResponse>({
    method: 'POST',
    endpoint: '/settings/update/transcription',
    payload: { transcription },
  })
  return reply.payload.settings
}

export const updateHistory = async (history: HistorySettings): Promise<AppSettings> => {
  const reply = await sendIpc<{ history: HistorySettings }, SettingsResponse>({
    method: 'POST',
    endpoint: '/settings/update/history',
    payload: { history },
  })
  return reply.payload.settings
}

export const getHistoryPage = async (page: number, page_size = 20): Promise<HistoryPage> => {
  const reply = await sendIpc<{ page: number; page_size: number }, HistoryResponse>({
    method: 'GET',
    endpoint: '/settings/history',
    payload: { page, page_size },
  })
  return reply.payload.history
}

export const deleteHistoryEntry = async (id: string): Promise<boolean> => {
  const reply = await sendIpc<{ id: string }, HistoryDeleteResponse>({
    method: 'POST',
    endpoint: '/settings/history/delete',
    payload: { id },
  })
  return reply.payload.deleted
}

export const getAboutLogsDir = async (): Promise<string> => {
  const reply = await sendIpc<Record<string, never>, AboutLogsDirResponse>({
    method: 'GET',
    endpoint: '/settings/about/logs_dir',
    payload: {},
  })
  return reply.payload.logs_dir
}

export const openAboutLogsDir = async (): Promise<string> => {
  const reply = await sendIpc<Record<string, never>, AboutLogsDirResponse>({
    method: 'POST',
    endpoint: '/settings/about/open_logs_dir',
    payload: {},
  })
  return reply.payload.logs_dir
}

export const openAboutExternalUrl = async (url: string): Promise<string> => {
  const reply = await sendIpc<{ url: string }, AboutOpenExternalUrlResponse>({
    method: 'POST',
    endpoint: '/settings/about/open_external_url',
    payload: { url },
  })
  return reply.payload.url
}

export const signalSettingsWindowReady = async (): Promise<void> => {
  await sendIpc<Record<string, never>, { visible: boolean }>({
    method: 'POST',
    endpoint: '/settings/window/ready',
    payload: {},
  })
}

export const resetDefaults = async (
  scope: 'general' | 'logging' | 'transcription' | 'history' | 'all',
): Promise<AppSettings> => {
  const reply = await sendIpc<{ scope: 'general' | 'logging' | 'transcription' | 'history' | 'all' }, SettingsResponse>({
    method: 'POST',
    endpoint: '/settings/reset/defaults',
    payload: { scope },
  })
  return reply.payload.settings
}

export const subscribeSettingsEvents = (
  onEvent: (event: SettingsEventEnvelope) => void,
): (() => void) => {
  const handler = (ev: Event) => {
    const custom = ev as CustomEvent<unknown>
    const detail = custom.detail as SettingsEventEnvelope | undefined
    if (!detail || typeof detail !== 'object' || typeof (detail as { event?: string }).event !== 'string') {
      return
    }
    onEvent(detail)
  }

  window.addEventListener('vocoflow:settings-event', handler as EventListener)
  return () => window.removeEventListener('vocoflow:settings-event', handler as EventListener)
}