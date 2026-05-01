use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::settings;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashMessage {
  pub message: String,
  pub occurred_at_unix_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SettingsFlashPayload {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub llm_post_process_error: Option<FlashMessage>,
}

fn flash_path() -> PathBuf {
  settings::data_dir().join("settings_flash.json")
}

fn now_unix_ms() -> u128 {
  std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map(|d| d.as_millis())
    .unwrap_or(0)
}

pub fn record_llm_post_process_error(message: String) -> Result<(), String> {
  let payload = SettingsFlashPayload {
    llm_post_process_error: Some(FlashMessage {
      message,
      occurred_at_unix_ms: now_unix_ms(),
    }),
  };

  let path = flash_path();
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent)
      .map_err(|e| format!("create flash dir failed: {e}"))?;
  }

  let json = serde_json::to_string_pretty(&payload)
    .map_err(|e| format!("serialize flash payload failed: {e}"))?;
  fs::write(path, json)
    .map_err(|e| format!("write flash payload failed: {e}"))?;

  crate::settings_window::bridge::events::emit_settings_flash(&payload);
  Ok(())
}

pub fn take_for_settings_flash() -> Result<Option<SettingsFlashPayload>, String>
{
  let path = flash_path();
  if !path.exists() {
    return Ok(None);
  }

  let raw = fs::read_to_string(&path)
    .map_err(|e| format!("read flash payload failed: {e}"))?;
  let parsed = serde_json::from_str::<SettingsFlashPayload>(&raw)
    .map_err(|e| format!("parse flash payload failed: {e}"))?;

  // Read-once semantics: clear after first successful read.
  let _ = fs::remove_file(&path);

  if parsed.llm_post_process_error.is_none() {
    Ok(None)
  } else {
    Ok(Some(parsed))
  }
}
