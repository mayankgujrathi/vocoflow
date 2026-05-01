use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::settings;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashMessage {
  pub message: String,
  pub occurred_at_unix_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashMarker {
  pub occurred_at_unix_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SettingsFlashPayload {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub llm_post_process_error: Option<FlashMessage>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub history_changed: Option<FlashMarker>,
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
  update_flash_payload(|payload| {
    payload.llm_post_process_error = Some(FlashMessage {
      message,
      occurred_at_unix_ms: now_unix_ms(),
    });
  })
}

pub fn record_history_changed() -> Result<(), String> {
  update_flash_payload(|payload| {
    payload.history_changed = Some(FlashMarker {
      occurred_at_unix_ms: now_unix_ms(),
    });
  })
}

fn update_flash_payload(
  mutator: impl FnOnce(&mut SettingsFlashPayload),
) -> Result<(), String> {
  let mut payload = read_flash_payload_for_update()?;
  mutator(&mut payload);

  write_flash_payload(&payload)
}

fn read_flash_payload_for_update() -> Result<SettingsFlashPayload, String> {
  let path = flash_path();
  if !path.exists() {
    return Ok(SettingsFlashPayload::default());
  }

  let raw = fs::read_to_string(&path)
    .map_err(|e| format!("read flash payload failed: {e}"))?;
  serde_json::from_str::<SettingsFlashPayload>(&raw)
    .map_err(|e| format!("parse flash payload failed: {e}"))
}

fn write_flash_payload(payload: &SettingsFlashPayload) -> Result<(), String> {
  let path = flash_path();

  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent)
      .map_err(|e| format!("create flash dir failed: {e}"))?;
  }

  let json = serde_json::to_string_pretty(&payload)
    .map_err(|e| format!("serialize flash payload failed: {e}"))?;
  fs::write(path, json)
    .map_err(|e| format!("write flash payload failed: {e}"))?;
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

  if parsed.llm_post_process_error.is_none() && parsed.history_changed.is_none()
  {
    Ok(None)
  } else {
    Ok(Some(parsed))
  }
}
