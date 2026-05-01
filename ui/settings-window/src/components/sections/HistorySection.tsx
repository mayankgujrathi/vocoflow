import type { HistoryPage, HistorySettings } from '../../types/settings'
import { FormField } from '../fields/FormField'

type HistorySectionProps = {
  settings: HistorySettings
  onRetentionChange: (retention_max_sessions: number) => void
  page: HistoryPage
  loading: boolean
  onPrevPage: () => void
  onNextPage: () => void
  onCopyRaw: (text: string) => void
  onCopyProcessed: (text: string | null | undefined) => void
  onDelete: (id: string) => void
}

const MAX_RETENTION = 200

export function HistorySection({
  settings,
  onRetentionChange,
  page,
  loading,
  onPrevPage,
  onNextPage,
  onCopyRaw,
  onCopyProcessed,
  onDelete,
}: HistorySectionProps) {
  return (
    <section className="space-y-3">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold text-slate-100">History</h2>
        <div className="text-xs text-slate-300">Stored locally only on this device.</div>
      </div>

      <FormField label="Retention (last N sessions)" description="0 means never store history. Higher values keep more sessions.">
        <div className="space-y-2">
          <input
            type="range"
            min={0}
            max={MAX_RETENTION}
            step={1}
            value={settings.retention_max_sessions}
            onChange={(e) => onRetentionChange(Number(e.target.value) || 0)}
            className="w-full accent-accent"
          />
          <div className="flex justify-between text-xs text-slate-400">
            <span>0 (never)</span>
            <span className="font-medium text-slate-200">{settings.retention_max_sessions}</span>
            <span>{MAX_RETENTION}</span>
          </div>
        </div>
      </FormField>

      <div className="rounded-xl border border-slate-700/60 bg-slate-900/50 p-3">
        <div className="mb-3 flex items-center justify-between text-xs text-slate-300">
          <span>
            Page {page.page} / {page.total_pages} • {page.total_items} total
          </span>
          <div className="flex gap-2">
            <button type="button" onClick={onPrevPage} disabled={page.page <= 1 || loading} className="rounded border border-slate-600 px-2 py-1 disabled:opacity-50">
              ◀ Prev
            </button>
            <button type="button" onClick={onNextPage} disabled={page.page >= page.total_pages || loading} className="rounded border border-slate-600 px-2 py-1 disabled:opacity-50">
              Next ▶
            </button>
          </div>
        </div>

        <div className="space-y-3">
          {page.items.length === 0 && <div className="text-xs text-slate-400">No history entries yet.</div>}
          {page.items.map((entry) => (
            (() => {
              const raw = entry.raw_transcript?.trim() || ''
              const processed = entry.processed_transcript?.trim() || ''
              const hasProcessed = processed.length > 0
              const sameRawAndProcessed = hasProcessed && raw === processed

              return (
                <div key={entry.id} className="rounded-md border border-slate-700/60 bg-slate-950/30 p-2.5">
                  <div className="mb-1.5 flex flex-wrap items-center justify-between gap-2 text-xs text-slate-400">
                    <span>{new Date(entry.created_at_unix_ms).toLocaleString()}</span>
                    <div className="flex flex-wrap items-center gap-3">
                      {sameRawAndProcessed ? (
                        <button type="button" onClick={() => onCopyRaw(entry.raw_transcript)} className="text-slate-300 hover:text-white">
                          Copy
                        </button>
                      ) : (
                        <>
                          <button type="button" onClick={() => onCopyRaw(entry.raw_transcript)} className="text-slate-300 hover:text-white">
                            Copy raw
                          </button>
                          {hasProcessed && (
                            <button type="button" onClick={() => onCopyProcessed(entry.processed_transcript)} className="text-slate-300 hover:text-white">
                              Copy processed
                            </button>
                          )}
                        </>
                      )}
                      <button type="button" onClick={() => onDelete(entry.id)} className="text-rose-300 hover:text-rose-200">
                        Delete
                      </button>
                    </div>
                  </div>

                  <audio controls preload="none" className="mb-1.5 h-8 w-full" src={`/settings/history_audio/${entry.id}`} />

                  <div className="space-y-1.5 text-xs text-slate-300">
                    {sameRawAndProcessed ? (
                      <div>
                        <div className="text-slate-400">raw + processed transcript</div>
                        <div className="max-h-16 overflow-auto whitespace-pre-wrap italic">{entry.raw_transcript}</div>
                      </div>
                    ) : (
                      <>
                        <div>
                          <div className="text-slate-400">raw transcript</div>
                          <div className="max-h-16 overflow-auto whitespace-pre-wrap italic">{entry.raw_transcript}</div>
                        </div>

                        {hasProcessed && (
                          <div>
                            <div className="text-slate-400">processed transcript</div>
                            <div className="max-h-16 overflow-auto whitespace-pre-wrap italic">{entry.processed_transcript}</div>
                          </div>
                        )}
                      </>
                    )}
                  </div>
                </div>
              )
            })()
          ))}
        </div>
      </div>
    </section>
  )
}
