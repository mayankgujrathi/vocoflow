# Settings and Logging

This page contains operational details for runtime settings, logs, and traces.

## Settings File

### History

- History is stored **locally on device only**.
- Settings includes a **retention slider** for last N sessions.
  - `0` means do not retain history.
  - Max retention is capped in-app for performance.
- History view supports:
  - Audio playback for each saved entry
  - Copy raw transcript (local ASR)
  - Copy processed transcript (LLM output when available)
  - Pagination to keep rendering lightweight

- Path: `<base_path>/settings.json`
- The app creates this file with defaults if it is missing.

## Settings Overview

The Settings UI groups options into **General**, **Logging**, and **Transcription**.

### General

- `start_on_login`: Launch Vocoflow automatically on user sign-in.
- `hotkey.binding`: Global trigger string supporting keyboard/mouse and chords (examples: ``Ctrl+ ` ``, `Ctrl+K, C`, `Ctrl+Mouse4`).
- `hotkey.chord_timeout_ms`: Max delay (ms) allowed between chord steps.

### Logging

- `logging.app_log_max_lines`: Max retained lines in `application.log`.
- `logging.trace_file_limit`: Max retained trace files.
- `logging.enable_debug_logs`: Enables verbose debug logging.

UI labels:

- **App log max lines**
- **Trace file limit**
- **Enable debug logs**

### Transcription

- `transcription.model_cache_ttl_secs`: Cache TTL for model metadata.
- `transcription.built_in_dictionary`: Built-in substitutions/vocabulary entries.
- `transcription.user_dictionary`: User-provided vocabulary entries.
- `transcription.transcript_reformatting_level`: `none | minimal | normal | freeform`.
- `transcription.llm_base_url`: OpenAI-compatible API base URL.
- `transcription.llm_model_name`: Model identifier.
- `transcription.llm_api_key`: Optional API key.
- `transcription.llm_custom_prompt`: Extra rewrite instructions.

UI labels:

- **Model cache TTL (secs)**
- **User dictionary**
- **Reformatting level**
- **LLM base URL**
- **LLM model name**
- **LLM API key**
- **LLM custom prompt**

## Reformatting Levels

- `none` (default): keep transcript unchanged; model calls are generally bypassed.
- `minimal`: light cleanup (punctuation, casing, small clarity edits).
- `normal`: readability/grammar improvements while preserving intent.
- `freeform`: most polished app-context-adapted rewrite; wording may change more.

LLM reformatting uses focused-app context (window title/app metadata) when enabled.

## Base Data Paths

- **Windows:** `C:\Users\<username>\AppData\Roaming\vocoflow\vocoflow\`
- **macOS:** `/Users/<username>/Library/Application Support/com.vocoflow.vocoflow/`
- **Linux:** `/home/<username>/.local/share/vocoflow/vocoflow/`

## Logs and Traces

- Application log: `<base_path>/logs/application.log`
- Trace directory: `<base_path>/logs/traces/`
- Trace file format: `trace-<timestamp>.json` (Chrome Trace Event format)

### Trace Viewers

- Perfetto UI: https://ui.perfetto.dev
- Chromium tracing: `chrome://tracing`

## Retention Defaults

- Application log: last **1000 lines**
- Trace files: latest **100 files**

## Log Levels

- Default: `info`
- Debug logs can be enabled in settings.

## Notes

- Logging settings are refreshed at runtime.

## Related Docs

- [Architecture](ARCHITECTURE.md)
- [Build and Release](BUILD_AND_RELEASE.md)
- [Licensing and Acknowledgments](LICENSES.md)
