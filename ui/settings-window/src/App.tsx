import { useEffect, useMemo, useRef, useState } from 'react'
import { FaCircleInfo, FaGear, FaMicrophoneLines, FaSliders } from 'react-icons/fa6'

import {
  getAllSettings,
  openAboutExternalUrl,
  openAboutLogsDir,
  resetDefaults,
  signalSettingsWindowReady,
  subscribeSettingsEvents,
  updateHotkey,
  updateLogging,
  updateStartOnLogin,
  updateTranscription,
} from './ipc'
import { SettingsShell } from './components/layout/SettingsShell'
import { SettingsSidebar, type SettingsTab } from './components/navigation/SettingsSidebar'
import { AboutSection } from './components/sections/AboutSection'
import { GeneralSection } from './components/sections/GeneralSection'
import { LoggingSection } from './components/sections/LoggingSection'
import { TranscriptionSection } from './components/sections/TranscriptionSection'
import type { AppSettings } from './types/settings'

const LLM_GUIDE_URL = 'https://github.com/mayankgujrathi/vocoflow/blob/main/docs/LLM_SETUP_AND_USAGE.md'

const EMPTY_SETTINGS: AppSettings = {
  start_on_login: false,
  hotkey: {
    binding: 'Ctrl+`',
    chord_timeout_ms: 1200,
    parsed: { normalized: 'Ctrl+`', sequence: [] },
  },
  logging: { app_log_max_lines: 1000, trace_file_limit: 100, enable_debug_logs: false },
  transcription: {
    built_in_dictionary: [],
    user_dictionary: [],
    model_cache_ttl_secs: 600,
    transcript_reformatting_level: 'none',
    llm_api_key: null,
    llm_base_url: '',
    llm_model_name: '',
    llm_custom_prompt: '',
  },
}

function App() {
  const [activeTab, setActiveTab] = useState<SettingsTab>('general')
  const [settings, setSettings] = useState<AppSettings>(EMPTY_SETTINGS)
  const [logsDir, setLogsDir] = useState('')
  const [status, setStatus] = useState('Loading settings...')
  const [flashError, setFlashError] = useState<string>('')
  const [savingKey, setSavingKey] = useState<string>('')
  const [uiBootReady, setUiBootReady] = useState(false)
  const lastEventVersionRef = useRef(0)
  const suppressAutosaveRef = useRef(false)
  const isClosingRef = useRef(false)
  const dirtyStartOnLoginRef = useRef(false)
  const dirtyHotkeyRef = useRef(false)
  const dirtyLoggingRef = useRef(false)
  const dirtyTranscriptionRef = useRef(false)
  const startOnLoginTimerRef = useRef<number | null>(null)
  const hotkeyTimerRef = useRef<number | null>(null)
  const loggingTimerRef = useRef<number | null>(null)
  const transcriptionTimerRef = useRef<number | null>(null)
  const prevStartOnLoginRef = useRef<boolean | null>(null)
  const prevHotkeyBindingRef = useRef<string | null>(null)
  const prevHotkeyTimeoutRef = useRef<number | null>(null)
  const prevLoggingSnapshotRef = useRef<string | null>(null)
  const prevTranscriptionSnapshotRef = useRef<string | null>(null)

  const applyRemoteSettings = (next: AppSettings) => {
    suppressAutosaveRef.current = true
    setSettings(next)
    prevStartOnLoginRef.current = next.start_on_login
    prevHotkeyBindingRef.current = next.hotkey.binding
    prevHotkeyTimeoutRef.current = next.hotkey.chord_timeout_ms
    prevLoggingSnapshotRef.current = JSON.stringify(next.logging)
    prevTranscriptionSnapshotRef.current = JSON.stringify(next.transcription)
    dirtyStartOnLoginRef.current = false
    dirtyHotkeyRef.current = false
    dirtyLoggingRef.current = false
    dirtyTranscriptionRef.current = false
    queueMicrotask(() => {
      suppressAutosaveRef.current = false
    })
  }

  const clearPendingAutosaveTimers = () => {
    if (startOnLoginTimerRef.current != null) {
      window.clearTimeout(startOnLoginTimerRef.current)
      startOnLoginTimerRef.current = null
    }
    if (hotkeyTimerRef.current != null) {
      window.clearTimeout(hotkeyTimerRef.current)
      hotkeyTimerRef.current = null
    }
    if (loggingTimerRef.current != null) {
      window.clearTimeout(loggingTimerRef.current)
      loggingTimerRef.current = null
    }
    if (transcriptionTimerRef.current != null) {
      window.clearTimeout(transcriptionTimerRef.current)
      transcriptionTimerRef.current = null
    }
  }

  const sidebarItems = useMemo(
    () => [
      { id: 'general' as const, label: 'General', icon: FaSliders },
      { id: 'logging' as const, label: 'Logging', icon: FaGear },
      { id: 'transcription' as const, label: 'Speech', icon: FaMicrophoneLines },
      { id: 'about' as const, label: 'About', icon: FaCircleInfo },
    ],
    [],
  )

  useEffect(() => {
    const load = async () => {
      try {
        const loadedSettingsPayload = await getAllSettings()
        applyRemoteSettings(loadedSettingsPayload.settings)
        setLogsDir(loadedSettingsPayload.logs_dir)
        const flash = loadedSettingsPayload.flash?.llm_post_process_error
        if (flash?.message) {
          setFlashError(flash.message)
        }
        setStatus('Settings loaded.')
        setUiBootReady(true)
      } catch (error) {
        setStatus(`Failed to load settings: ${String(error)}`)
        // Even on error, reveal UI so user can see the status message.
        setUiBootReady(true)
      }
    }
    void load()
  }, [])

  useEffect(() => {
    if (!uiBootReady) {
      return
    }

    const root = document.getElementById('root')
    const splash = document.getElementById('boot-splash')

    requestAnimationFrame(() => {
      requestAnimationFrame(() => {
        if (root) {
          root.style.opacity = '1'
        }
        if (splash) {
          splash.remove()
        }
        void signalSettingsWindowReady().catch(() => {
          // Best-effort signal for native window show timing.
        })
      })
    })
  }, [uiBootReady])

  useEffect(() => {
    return subscribeSettingsEvents((evt) => {
      if (evt.version <= lastEventVersionRef.current) {
        return
      }
      lastEventVersionRef.current = evt.version

      if (evt.event === 'settings.changed') {
        applyRemoteSettings(evt.payload.settings)
      }

      if (evt.event === 'settings.flash') {
        const flash = evt.payload.flash.llm_post_process_error
        if (flash?.message) {
          setFlashError(flash.message)
        }
      }
    })
  }, [])

  useEffect(() => {
    const markClosing = () => {
      isClosingRef.current = true
      clearPendingAutosaveTimers()
    }

    window.addEventListener('beforeunload', markClosing)
    window.addEventListener('pagehide', markClosing)
    document.addEventListener('visibilitychange', () => {
      if (document.visibilityState === 'hidden') {
        markClosing()
      }
    })

    return () => {
      markClosing()
      window.removeEventListener('beforeunload', markClosing)
      window.removeEventListener('pagehide', markClosing)
    }
  }, [])

  useEffect(() => {
    if (suppressAutosaveRef.current) {
      return
    }
    if (isClosingRef.current || !dirtyStartOnLoginRef.current) {
      return
    }
    if (prevStartOnLoginRef.current === settings.start_on_login) {
      return
    }

    prevStartOnLoginRef.current = settings.start_on_login
    startOnLoginTimerRef.current = window.setTimeout(async () => {
      if (isClosingRef.current) {
        return
      }
      setSavingKey('general')
      try {
        await updateStartOnLogin(settings.start_on_login)
        dirtyStartOnLoginRef.current = false
        setStatus('Start-on-login setting auto-saved.')
      } catch (error) {
        setStatus(`Save failed: ${String(error)}`)
      } finally {
        setSavingKey('')
      }
    }, 250)
    return () => {
      if (startOnLoginTimerRef.current != null) {
        window.clearTimeout(startOnLoginTimerRef.current)
        startOnLoginTimerRef.current = null
      }
    }
  }, [settings.start_on_login])

  useEffect(() => {
    if (suppressAutosaveRef.current) {
      return
    }
    if (isClosingRef.current || !dirtyHotkeyRef.current) {
      return
    }
    if (
      prevHotkeyBindingRef.current === settings.hotkey.binding
      && prevHotkeyTimeoutRef.current === settings.hotkey.chord_timeout_ms
    ) {
      return
    }

    prevHotkeyBindingRef.current = settings.hotkey.binding
    prevHotkeyTimeoutRef.current = settings.hotkey.chord_timeout_ms
    hotkeyTimerRef.current = window.setTimeout(async () => {
      if (isClosingRef.current) {
        return
      }
      setSavingKey('general')
      try {
        await updateHotkey(settings.hotkey.binding, settings.hotkey.chord_timeout_ms)
        dirtyHotkeyRef.current = false
        setStatus('Hotkey setting auto-saved.')
      } catch (error) {
        setStatus(`Save failed: ${String(error)}`)
      } finally {
        setSavingKey('')
      }
    }, 250)
    return () => {
      if (hotkeyTimerRef.current != null) {
        window.clearTimeout(hotkeyTimerRef.current)
        hotkeyTimerRef.current = null
      }
    }
  }, [settings.hotkey.binding, settings.hotkey.chord_timeout_ms])

  useEffect(() => {
    if (suppressAutosaveRef.current) {
      return
    }
    if (isClosingRef.current || !dirtyLoggingRef.current) {
      return
    }
    const loggingSnapshot = JSON.stringify(settings.logging)
    if (prevLoggingSnapshotRef.current === loggingSnapshot) {
      return
    }
    prevLoggingSnapshotRef.current = loggingSnapshot

    loggingTimerRef.current = window.setTimeout(async () => {
      if (isClosingRef.current) {
        return
      }
      setSavingKey('logging')
      try {
        await updateLogging(settings.logging)
        dirtyLoggingRef.current = false
        setStatus('Logging settings auto-saved.')
      } catch (error) {
        setStatus(`Save failed: ${String(error)}`)
      } finally {
        setSavingKey('')
      }
    }, 300)
    return () => {
      if (loggingTimerRef.current != null) {
        window.clearTimeout(loggingTimerRef.current)
        loggingTimerRef.current = null
      }
    }
  }, [settings.logging])

  useEffect(() => {
    if (suppressAutosaveRef.current) {
      return
    }
    if (isClosingRef.current || !dirtyTranscriptionRef.current) {
      return
    }
    const transcriptionSnapshot = JSON.stringify(settings.transcription)
    if (prevTranscriptionSnapshotRef.current === transcriptionSnapshot) {
      return
    }
    prevTranscriptionSnapshotRef.current = transcriptionSnapshot

    transcriptionTimerRef.current = window.setTimeout(async () => {
      if (isClosingRef.current) {
        return
      }
      setSavingKey('transcription')
      try {
        await updateTranscription(settings.transcription)
        dirtyTranscriptionRef.current = false
        setStatus('Transcription settings auto-saved.')
      } catch (error) {
        setStatus(`Save failed: ${String(error)}`)
      } finally {
        setSavingKey('')
      }
    }, 500)
    return () => {
      if (transcriptionTimerRef.current != null) {
        window.clearTimeout(transcriptionTimerRef.current)
        transcriptionTimerRef.current = null
      }
    }
  }, [settings.transcription])

  const openLogs = async () => {
    setSavingKey('about')
    try {
      const dir = await openAboutLogsDir()
      setLogsDir(dir)
      setStatus('Opened logs directory.')
    } catch (error) {
      setStatus(`Open logs failed: ${String(error)}`)
    } finally {
      setSavingKey('')
    }
  }

  const openExternalLink = async (url: string) => {
    try {
      await openAboutExternalUrl(url)
      setStatus('Opened link in default browser.')
    } catch (error) {
      setStatus(`Open link failed: ${String(error)}`)
    }
  }

  const runReset = async (scope: 'general' | 'logging' | 'transcription' | 'all') => {
    setSavingKey(scope)
    try {
      const next = await resetDefaults(scope)
      setSettings(next)
      setStatus(`Reset ${scope} defaults.`)
    } catch (error) {
      setStatus(`Reset failed: ${String(error)}`)
    } finally {
      setSavingKey('')
    }
  }

  return (
    <SettingsShell sidebar={<SettingsSidebar items={sidebarItems} activeTab={activeTab} onSelect={setActiveTab} />}>
      <header className="mb-5 rounded-xl border border-slate-700/50 bg-material-gradient p-4">
        <div className="flex items-center justify-between gap-3">
          <div>
            <h1 className="text-xl font-semibold">Vocoflow Settings</h1>
            <p className="mt-1 text-xs text-slate-200/90"></p>
          </div>
          <button type="button" onClick={() => void runReset('all')} className="rounded-lg border border-slate-500/70 bg-slate-900/40 px-3 py-1.5 text-xs text-slate-100">
            Reset all defaults
          </button>
        </div>
      </header>

      <div className="space-y-5">
        {flashError && (
          <div className="rounded-lg border border-amber-400/50 bg-amber-950/40 px-3 py-2 text-sm text-amber-100">
            <div className="font-semibold">Last-run LLM post-processing error</div>
            <div className="mt-1 whitespace-pre-wrap text-xs text-amber-200/95">{flashError}</div>
            <button
              type="button"
              onClick={() => void openExternalLink(LLM_GUIDE_URL)}
              className="mt-2 rounded border border-amber-300/60 px-2 py-1 text-xs text-amber-100 hover:bg-amber-400/10"
            >
              Learn more: LLM setup and usage
            </button>
          </div>
        )}

        {activeTab === 'general' && (
          <GeneralSection
            startOnLogin={settings.start_on_login}
            hotkeyBinding={settings.hotkey.binding}
            onChange={(next) => {
              dirtyStartOnLoginRef.current = true
              setSettings((prev) => ({ ...prev, start_on_login: next }))
            }}
            onHotkeyBindingChange={(next) => {
              dirtyHotkeyRef.current = true
              setSettings((prev) => ({
                ...prev,
                hotkey: { ...prev.hotkey, binding: next },
              }))
            }}
            onReset={() => void runReset('general')}
            saving={savingKey === 'general'}
          />
        )}

        {activeTab === 'logging' && (
          <LoggingSection
            value={settings.logging}
            onChange={(next) => {
              dirtyLoggingRef.current = true
              setSettings((prev) => ({ ...prev, logging: next }))
            }}
            onReset={() => void runReset('logging')}
            saving={savingKey === 'logging'}
          />
        )}

        {activeTab === 'transcription' && (
          <TranscriptionSection
            value={settings.transcription}
            onChange={(next) => {
              dirtyTranscriptionRef.current = true
              setSettings((prev) => ({ ...prev, transcription: next }))
            }}
            onReset={() => void runReset('transcription')}
            onOpenLlmGuide={() => void openExternalLink(LLM_GUIDE_URL)}
            saving={savingKey === 'transcription'}
          />
        )}

        {activeTab === 'about' && (
          <AboutSection
            logsDir={logsDir}
            onOpenLogsDir={openLogs}
            onOpenExternalUrl={(url) => void openExternalLink(url)}
            opening={savingKey === 'about'}
          />
        )}
      </div>

      <footer className="mt-6 rounded-lg border border-slate-700/60 bg-slate-900/40 px-3 py-2 text-xs text-slate-300">{status}</footer>
    </SettingsShell>
  )
}

export default App
