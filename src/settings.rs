use std::fs;
use std::path::PathBuf;
use std::sync::{OnceLock, RwLock};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::app::{
  DEFAULT_LLM_BASE_URL, DEFAULT_LLM_CUSTOM_PROMPT, DEFAULT_LLM_MODEL_NAME,
};
use crate::hotkey::{ParsedHotkey, parse_hotkey_binding};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptReformattingLevel {
  #[default]
  None,
  Minimal,
  Normal,
  #[serde(alias = "actionable")]
  Freeform,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct AppSettings {
  pub start_on_login: bool,
  pub hotkey: HotkeySettings,
  pub logging: LoggingSettings,
  pub transcription: TranscriptionSettings,
  pub history: HistorySettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct HistorySettings {
  pub retention_max_sessions: u32,
}

impl Default for HistorySettings {
  fn default() -> Self {
    Self {
      retention_max_sessions: 20,
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct HotkeySettings {
  pub binding: String,
  pub parsed: ParsedHotkey,
  pub chord_timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct LoggingSettings {
  pub app_log_max_lines: usize,
  pub trace_file_limit: usize,
  pub enable_debug_logs: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct TranscriptionSettings {
  pub built_in_dictionary: Vec<String>,
  pub user_dictionary: Vec<String>,
  pub model_cache_ttl_secs: u64,
  pub transcript_reformatting_level: TranscriptReformattingLevel,
  pub llm_api_key: Option<String>,
  pub llm_base_url: String,
  pub llm_model_name: String,
  pub llm_custom_prompt: String,
}

impl Default for TranscriptionSettings {
  fn default() -> Self {
    Self {
      built_in_dictionary: Vec::new(),
      user_dictionary: Vec::new(),
      model_cache_ttl_secs: 10 * 60,
      transcript_reformatting_level: TranscriptReformattingLevel::None,
      llm_api_key: None,
      llm_base_url: DEFAULT_LLM_BASE_URL.to_owned(),
      llm_model_name: DEFAULT_LLM_MODEL_NAME.to_owned(),
      llm_custom_prompt: DEFAULT_LLM_CUSTOM_PROMPT.to_owned(),
    }
  }
}

impl Default for LoggingSettings {
  fn default() -> Self {
    Self {
      app_log_max_lines: 1000,
      trace_file_limit: 100,
      enable_debug_logs: false,
    }
  }
}

impl Default for HotkeySettings {
  fn default() -> Self {
    let binding = "Ctrl+`".to_string();
    let parsed = parse_hotkey_binding(&binding).unwrap_or(ParsedHotkey {
      normalized: "Ctrl+`".to_string(),
      sequence: Vec::new(),
    });
    Self {
      binding,
      parsed,
      chord_timeout_ms: 1200,
    }
  }
}

static SETTINGS: OnceLock<RwLock<AppSettings>> = OnceLock::new();

pub fn data_dir() -> PathBuf {
  ProjectDirs::from("com", "vocoflow", "vocoflow")
    .map(|dirs| dirs.data_dir().to_path_buf())
    .unwrap_or_else(|| {
      std::env::current_dir().unwrap_or_else(|_| std::env::temp_dir())
    })
}

pub fn settings_path() -> PathBuf {
  data_dir().join("settings.json")
}

fn write_default_settings(path: &std::path::Path) -> Result<(), String> {
  write_settings(path, &AppSettings::default())
}

fn write_settings(
  path: &std::path::Path,
  settings: &AppSettings,
) -> Result<(), String> {
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent)
      .map_err(|e| format!("create settings dir failed: {e}"))?;
  }
  let json = serde_json::to_string_pretty(settings)
    .map_err(|e| format!("serialize settings failed: {e}"))?;
  fs::write(path, json).map_err(|e| format!("write settings failed: {e}"))
}

fn load_from_disk() -> Result<AppSettings, String> {
  let path = settings_path();
  if !path.exists() {
    write_default_settings(&path)?;
    return Ok(AppSettings::default());
  }

  let raw = fs::read_to_string(&path)
    .map_err(|e| format!("read settings failed: {e}"))?;
  parse_and_backfill_settings(raw.as_str(), &path)
}

fn parse_and_backfill_settings(
  raw: &str,
  path: &std::path::Path,
) -> Result<AppSettings, String> {
  let parsed = serde_json::from_str::<AppSettings>(raw)
    .map_err(|e| format!("parse settings failed: {e}"))?;

  // Backfill newly added/defaulted keys into the on-disk settings file.
  if let Ok(raw_value) = serde_json::from_str::<serde_json::Value>(raw) {
    let normalized_value = serde_json::to_value(&parsed)
      .map_err(|e| format!("serialize settings for migration failed: {e}"))?;

    if raw_value != normalized_value {
      write_settings(path, &parsed)?;
    }
  }

  Ok(parsed)
}

pub fn initialize() {
  let initial = load_from_disk().unwrap_or_default();
  let _ = SETTINGS.get_or_init(|| RwLock::new(initial));
}

pub fn current() -> AppSettings {
  initialize();
  match SETTINGS.get() {
    Some(lock) => lock.read().map(|g| g.clone()).unwrap_or_default(),
    None => AppSettings::default(),
  }
}

pub fn refresh_from_disk() -> Result<bool, String> {
  initialize();
  let next = load_from_disk()?;

  let Some(lock) = SETTINGS.get() else {
    return Ok(false);
  };

  let mut guard = lock
    .write()
    .map_err(|_| "settings lock poisoned".to_string())?;
  if *guard == next {
    return Ok(false);
  }

  *guard = next;
  Ok(true)
}

pub fn refresh_from_disk_best_effort(caller: &str) {
  match refresh_from_disk() {
    Ok(changed) => {
      debug!(caller, changed, "settings refresh attempted");
    }
    Err(e) => {
      tracing::warn!(caller, error = %e, "settings refresh failed; continuing with in-memory snapshot");
    }
  }
}

pub fn update_start_on_login(
  start_on_login: bool,
) -> Result<AppSettings, String> {
  info!(start_on_login, "updating start_on_login setting");
  initialize();
  let Some(lock) = SETTINGS.get() else {
    return Err("settings store unavailable".to_string());
  };

  let snapshot = {
    let mut guard = lock
      .write()
      .map_err(|_| "settings lock poisoned".to_string())?;
    guard.start_on_login = start_on_login;
    guard.clone()
  };

  write_settings(&settings_path(), &snapshot)?;
  let changed = refresh_from_disk()?;
  debug!(
    changed,
    "settings reloaded from disk after start_on_login update"
  );
  Ok(current())
}

pub fn update_hotkey(
  binding: String,
  chord_timeout_ms: Option<u64>,
) -> Result<AppSettings, String> {
  let parsed = parse_hotkey_binding(&binding)?;
  initialize();
  let Some(lock) = SETTINGS.get() else {
    return Err("settings store unavailable".to_string());
  };

  let snapshot = {
    let mut guard = lock
      .write()
      .map_err(|_| "settings lock poisoned".to_string())?;
    guard.hotkey.binding = binding;
    guard.hotkey.parsed = parsed;
    if let Some(timeout) = chord_timeout_ms {
      guard.hotkey.chord_timeout_ms = timeout.clamp(100, 5000);
    }
    guard.clone()
  };

  write_settings(&settings_path(), &snapshot)?;
  let _ = refresh_from_disk()?;
  Ok(current())
}

pub fn persist_start_on_login_from_system(
  start_on_login: bool,
) -> Result<AppSettings, String> {
  initialize();
  let Some(lock) = SETTINGS.get() else {
    return Err("settings store unavailable".to_string());
  };

  let snapshot = {
    let mut guard = lock
      .write()
      .map_err(|_| "settings lock poisoned".to_string())?;
    if guard.start_on_login == start_on_login {
      return Ok(guard.clone());
    }
    guard.start_on_login = start_on_login;
    guard.clone()
  };

  write_settings(&settings_path(), &snapshot)?;
  let _ = refresh_from_disk()?;
  Ok(current())
}

pub fn update_logging(logging: LoggingSettings) -> Result<AppSettings, String> {
  info!(
    app_log_max_lines = logging.app_log_max_lines,
    trace_file_limit = logging.trace_file_limit,
    enable_debug_logs = logging.enable_debug_logs,
    "updating logging settings"
  );
  initialize();
  let Some(lock) = SETTINGS.get() else {
    return Err("settings store unavailable".to_string());
  };

  let snapshot = {
    let mut guard = lock
      .write()
      .map_err(|_| "settings lock poisoned".to_string())?;
    guard.logging = logging;
    guard.clone()
  };

  write_settings(&settings_path(), &snapshot)?;
  let changed = refresh_from_disk()?;
  debug!(changed, "settings reloaded from disk after logging update");
  Ok(current())
}

pub fn update_transcription(
  transcription: TranscriptionSettings,
) -> Result<AppSettings, String> {
  info!(
    model_cache_ttl_secs = transcription.model_cache_ttl_secs,
    llm_base_url = %transcription.llm_base_url,
    llm_model_name = %transcription.llm_model_name,
    "updating transcription settings"
  );
  initialize();
  let Some(lock) = SETTINGS.get() else {
    return Err("settings store unavailable".to_string());
  };

  let snapshot = {
    let mut guard = lock
      .write()
      .map_err(|_| "settings lock poisoned".to_string())?;
    guard.transcription = transcription;
    guard.clone()
  };

  write_settings(&settings_path(), &snapshot)?;
  let changed = refresh_from_disk()?;
  debug!(
    changed,
    "settings reloaded from disk after transcription update"
  );
  Ok(current())
}

pub fn update_history(history: HistorySettings) -> Result<AppSettings, String> {
  info!(
    retention_max_sessions = history.retention_max_sessions,
    "updating history settings"
  );
  initialize();
  let Some(lock) = SETTINGS.get() else {
    return Err("settings store unavailable".to_string());
  };

  let snapshot = {
    let mut guard = lock
      .write()
      .map_err(|_| "settings lock poisoned".to_string())?;
    guard.history = HistorySettings {
      retention_max_sessions: history.retention_max_sessions.min(200),
    };
    guard.clone()
  };

  write_settings(&settings_path(), &snapshot)?;
  let changed = refresh_from_disk()?;
  debug!(changed, "settings reloaded from disk after history update");
  Ok(current())
}

pub fn reset_general_defaults() -> Result<AppSettings, String> {
  let defaults = AppSettings::default();
  initialize();
  let Some(lock) = SETTINGS.get() else {
    return Err("settings store unavailable".to_string());
  };

  let snapshot = {
    let mut guard = lock
      .write()
      .map_err(|_| "settings lock poisoned".to_string())?;
    guard.start_on_login = defaults.start_on_login;
    guard.hotkey = defaults.hotkey;
    guard.clone()
  };

  write_settings(&settings_path(), &snapshot)?;
  let _ = refresh_from_disk()?;
  Ok(current())
}

pub fn reset_logging_default() -> Result<AppSettings, String> {
  update_logging(LoggingSettings::default())
}

pub fn reset_transcription_default() -> Result<AppSettings, String> {
  update_transcription(TranscriptionSettings::default())
}

pub fn reset_history_default() -> Result<AppSettings, String> {
  update_history(HistorySettings::default())
}

pub fn reset_all_defaults() -> Result<AppSettings, String> {
  initialize();
  let defaults = AppSettings::default();
  let Some(lock) = SETTINGS.get() else {
    return Err("settings store unavailable".to_string());
  };

  {
    let mut guard = lock
      .write()
      .map_err(|_| "settings lock poisoned".to_string())?;
    *guard = defaults.clone();
  }

  write_settings(&settings_path(), &defaults)?;
  let _ = refresh_from_disk()?;
  Ok(current())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_parse_and_backfill_settings_writes_missing_transcription() {
    let unique = format!(
      "vocoflow_settings_test_{}",
      std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
    );

    let dir = std::env::temp_dir().join(unique);
    let file = dir.join("settings.json");
    fs::create_dir_all(&dir).expect("should create temp test dir");

    let old = r#"{
  "logging": {
    "app_log_max_lines": 1000,
    "trace_file_limit": 100,
    "enable_debug_logs": false
  }
}"#;
    fs::write(&file, old).expect("should write old settings json");

    let parsed = parse_and_backfill_settings(old, &file)
      .expect("should parse and backfill settings");
    assert_eq!(parsed.transcription, TranscriptionSettings::default());

    let updated =
      fs::read_to_string(&file).expect("should read backfilled settings");
    assert!(updated.contains("\"transcription\""));
    assert!(updated.contains("\"built_in_dictionary\""));
    assert!(updated.contains("\"user_dictionary\""));
    assert!(updated.contains("\"model_cache_ttl_secs\""));
    assert!(updated.contains("\"transcript_reformatting_level\""));
    assert!(updated.contains("\"llm_api_key\""));
    assert!(updated.contains("\"llm_base_url\""));
    assert!(updated.contains("\"llm_model_name\""));
    assert!(updated.contains("\"llm_custom_prompt\""));

    let _ = fs::remove_file(&file);
    let _ = fs::remove_dir_all(&dir);
  }
}
