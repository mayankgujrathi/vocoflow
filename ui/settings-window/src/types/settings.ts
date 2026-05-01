export type TranscriptReformattingLevel = 'none' | 'minimal' | 'normal' | 'freeform'

export type LoggingSettings = {
  app_log_max_lines: number
  trace_file_limit: number
  enable_debug_logs: boolean
}

export type TranscriptionSettings = {
  built_in_dictionary: string[]
  user_dictionary: string[]
  model_cache_ttl_secs: number
  transcript_reformatting_level: TranscriptReformattingLevel
  llm_api_key: string | null
  llm_base_url: string
  llm_model_name: string
  llm_custom_prompt: string
}

export type HistorySettings = {
  retention_max_sessions: number
}

export type HistoryEntry = {
  id: string
  created_at_unix_ms: number
  audio_file_name: string
  raw_transcript: string
  processed_transcript?: string | null
}

export type HistoryPage = {
  items: HistoryEntry[]
  page: number
  page_size: number
  total_items: number
  total_pages: number
}

export type HotkeySettings = {
  binding: string
  chord_timeout_ms: number
  parsed: {
    normalized: string
    sequence: Array<unknown>
  }
}

export type AppSettings = {
  start_on_login: boolean
  hotkey: HotkeySettings
  logging: LoggingSettings
  transcription: TranscriptionSettings
  history: HistorySettings
}

export type FlashMessage = {
  message: string
  occurred_at_unix_ms: number
}

export type FlashMarker = {
  occurred_at_unix_ms: number
}

export type SettingsFlashPayload = {
  llm_post_process_error?: FlashMessage
  history_changed?: FlashMarker
}
