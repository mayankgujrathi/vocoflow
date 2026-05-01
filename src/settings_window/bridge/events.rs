use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use serde::Serialize;
use tracing::warn;

use crate::runtime_flash;
use crate::settings;

static SETTINGS_EVENT_VERSION: AtomicU64 = AtomicU64::new(0);
static LAST_SETTINGS_SNAPSHOT: OnceLock<Mutex<Option<String>>> =
  OnceLock::new();

#[derive(Debug, Serialize)]
struct EventEnvelope<T: Serialize> {
  event: &'static str,
  version: u64,
  updated_at_ms: u128,
  payload: T,
}

#[derive(Debug, Serialize)]
struct SettingsChangedPayload {
  settings: settings::AppSettings,
}

#[derive(Debug, Serialize)]
struct SettingsFlashPayload {
  flash: runtime_flash::SettingsFlashPayload,
}

fn now_unix_ms() -> u128 {
  std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map(|d| d.as_millis())
    .unwrap_or(0)
}

fn next_version() -> u64 {
  SETTINGS_EVENT_VERSION.fetch_add(1, Ordering::AcqRel) + 1
}

fn emit_json_event<T: Serialize>(event: &'static str, payload: T) {
  let envelope = EventEnvelope {
    event,
    version: next_version(),
    updated_at_ms: now_unix_ms(),
    payload,
  };

  let Ok(serialized) = serde_json::to_string(&envelope) else {
    warn!(event, "failed to serialize settings event payload");
    return;
  };

  let script = format!(
    "window.dispatchEvent(new CustomEvent('vocoflow:settings-event', {{ detail: {} }}));",
    serialized
  );
  crate::settings_window::push_settings_window_script(script);
}

pub fn emit_settings_changed(settings: &settings::AppSettings) {
  let Ok(snapshot) = serde_json::to_string(settings) else {
    warn!("failed to serialize settings snapshot for dedupe");
    return;
  };

  let dedupe = LAST_SETTINGS_SNAPSHOT.get_or_init(|| Mutex::new(None));
  if let Ok(mut guard) = dedupe.lock() {
    if guard.as_ref() == Some(&snapshot) {
      return;
    }
    *guard = Some(snapshot);
  }

  emit_json_event(
    "settings.changed",
    SettingsChangedPayload {
      settings: settings.clone(),
    },
  );
}

pub fn emit_settings_flash(flash: &runtime_flash::SettingsFlashPayload) {
  emit_json_event(
    "settings.flash",
    SettingsFlashPayload {
      flash: flash.clone(),
    },
  );
}
