use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::settings;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
  pub id: String,
  pub created_at_unix_ms: u128,
  pub audio_file_name: String,
  pub raw_transcript: String,
  pub processed_transcript: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
struct HistoryStore {
  entries: Vec<HistoryEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HistoryPage {
  pub items: Vec<HistoryEntry>,
  pub page: u32,
  pub page_size: u32,
  pub total_items: u32,
  pub total_pages: u32,
}

pub fn history_path() -> PathBuf {
  settings::data_dir().join("history.json")
}

pub fn history_audio_dir() -> PathBuf {
  settings::data_dir().join("history_audio")
}

fn read_store() -> HistoryStore {
  let path = history_path();
  let Ok(raw) = fs::read_to_string(path) else {
    return HistoryStore::default();
  };
  let mut store =
    serde_json::from_str::<HistoryStore>(&raw).unwrap_or_default();
  sanitize_store(&mut store);
  store
}

fn write_store(store: &HistoryStore) -> Result<(), String> {
  let path = history_path();
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent)
      .map_err(|e| format!("create history dir failed: {e}"))?;
  }
  let raw = serde_json::to_string_pretty(store)
    .map_err(|e| format!("serialize history failed: {e}"))?;
  fs::write(path, raw).map_err(|e| format!("write history failed: {e}"))
}

fn now_unix_ms() -> u128 {
  std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map(|d| d.as_millis())
    .unwrap_or(0)
}

fn next_id() -> String {
  format!(
    "h_{}",
    std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .map(|d| d.as_nanos())
      .unwrap_or(0)
  )
}

pub fn resolve_audio_path_for_id(id: &str) -> Option<PathBuf> {
  let store = read_store();
  store
    .entries
    .iter()
    .find(|e| e.id == id)
    .map(|e| history_audio_dir().join(&e.audio_file_name))
    .filter(|p| p.is_file())
}

pub fn append_entry(
  recording_path: &Path,
  raw_transcript: &str,
  processed_transcript: Option<String>,
) -> Result<(), String> {
  if !is_valid_transcript(raw_transcript) {
    debug!("history append skipped: transcript failed validity checks");
    return Ok(());
  }

  let id = next_id();
  let audio_file_name = format!("{id}.wav");
  let audio_dir = history_audio_dir();
  fs::create_dir_all(&audio_dir)
    .map_err(|e| format!("create history audio dir failed: {e}"))?;
  let dst = audio_dir.join(&audio_file_name);
  fs::copy(recording_path, &dst)
    .map_err(|e| format!("copy recording to history failed: {e}"))?;

  let mut store = read_store();
  store.entries.insert(
    0,
    HistoryEntry {
      id,
      created_at_unix_ms: now_unix_ms(),
      audio_file_name,
      raw_transcript: raw_transcript.to_owned(),
      processed_transcript,
    },
  );

  enforce_retention_in_store(&mut store);
  write_store(&store)
}

fn is_valid_transcript(raw: &str) -> bool {
  let trimmed = raw.trim();
  if trimmed.is_empty() {
    return false;
  }

  let alnum_count = trimmed.chars().filter(|c| c.is_alphanumeric()).count();
  alnum_count >= 2
}

fn sanitize_store(store: &mut HistoryStore) {
  let audio_dir = history_audio_dir();
  let mut kept = Vec::with_capacity(store.entries.len());
  for entry in &store.entries {
    let audio_path = audio_dir.join(&entry.audio_file_name);
    if !is_valid_transcript(&entry.raw_transcript) || !audio_path.is_file() {
      let _ = fs::remove_file(audio_path);
      continue;
    }
    kept.push(entry.clone());
  }
  store.entries = kept;
}

fn enforce_retention_in_store(store: &mut HistoryStore) {
  let max = settings::current().history.retention_max_sessions as usize;
  if max == 0 {
    // Never retain history.
    let audio_dir = history_audio_dir();
    for entry in &store.entries {
      let _ = fs::remove_file(audio_dir.join(&entry.audio_file_name));
    }
    store.entries.clear();
    return;
  }

  if store.entries.len() <= max {
    return;
  }

  let audio_dir = history_audio_dir();
  for old in store.entries.iter().skip(max) {
    let _ = fs::remove_file(audio_dir.join(&old.audio_file_name));
  }
  store.entries.truncate(max);
}

pub fn enforce_retention_from_settings() {
  let mut store = read_store();
  enforce_retention_in_store(&mut store);
  if let Err(e) = write_store(&store) {
    warn!(error = %e, "failed writing history after retention enforcement");
  }
}

pub fn cleanup_invalid_entries() -> Result<(), String> {
  let mut store = read_store();
  sanitize_store(&mut store);
  write_store(&store)
}

pub fn delete_entry(id: &str) -> Result<bool, String> {
  let mut store = read_store();
  let before = store.entries.len();
  let audio_dir = history_audio_dir();
  store.entries.retain(|entry| {
    if entry.id == id {
      let _ = fs::remove_file(audio_dir.join(&entry.audio_file_name));
      false
    } else {
      true
    }
  });
  if store.entries.len() == before {
    return Ok(false);
  }
  write_store(&store)?;
  Ok(true)
}

pub fn list_page(page: u32, page_size: u32) -> HistoryPage {
  let store = read_store();
  let safe_page = page.max(1);
  let safe_size = page_size.clamp(1, 100);
  let total_items = store.entries.len() as u32;
  let total_pages = if total_items == 0 {
    1
  } else {
    ((total_items as f64) / (safe_size as f64)).ceil() as u32
  };
  let start = ((safe_page - 1) * safe_size) as usize;
  let end = (start + safe_size as usize).min(store.entries.len());
  let items = if start >= store.entries.len() {
    Vec::new()
  } else {
    store.entries[start..end].to_vec()
  };

  debug!(
    page = safe_page,
    page_size = safe_size,
    total_items,
    "history page listed"
  );

  HistoryPage {
    items,
    page: safe_page,
    page_size: safe_size,
    total_items,
    total_pages,
  }
}
