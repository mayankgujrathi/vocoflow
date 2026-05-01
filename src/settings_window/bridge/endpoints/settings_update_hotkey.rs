use serde::Deserialize;
use tracing::error;

use crate::decode_payload;
use crate::settings;
use crate::settings_window::bridge::lib::{
  BridgeHttpResponse, BridgeRequest, ResolvedRoute, make_error,
  success_response,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SettingsUpdateHotkeyRequest {
  binding: String,
  #[serde(default)]
  chord_timeout_ms: Option<u64>,
}

#[derive(Debug, serde::Serialize)]
struct SettingsUpdateHotkeyResponse {
  settings: settings::AppSettings,
}

pub fn handle(
  req: &BridgeRequest,
  route: &ResolvedRoute,
) -> BridgeHttpResponse {
  let payload: SettingsUpdateHotkeyRequest = match decode_payload!(
    req,
    SettingsUpdateHotkeyRequest,
    &route.route_kind
  ) {
    Ok(payload) => payload,
    Err(response) => return response,
  };

  match settings::update_hotkey(payload.binding, payload.chord_timeout_ms) {
    Ok(updated_settings) => {
      crate::settings_window::bridge::events::emit_settings_changed(
        &updated_settings,
      );
      success_response(
        req.request_id.clone(),
        route,
        SettingsUpdateHotkeyResponse {
          settings: updated_settings,
        },
      )
    }
    Err(err) => {
      error!(
        request_id = ?req.request_id,
        error = %err,
        "settings update hotkey failed"
      );
      let error = make_error(
        "SETTINGS_UPDATE_FAILED",
        format!("Failed to update hotkey settings: {err}"),
        Some("binding"),
        Some("hotkey string like 'Ctrl+`' or 'Ctrl+K, C'"),
        None,
      );
      let body = serde_json::json!({
        "request_id": req.request_id.clone(),
        "ok": false,
        "kind": "error.settings_update_failed",
        "payload": {},
        "error": error,
      })
      .to_string();
      BridgeHttpResponse { status: 500, body }
    }
  }
}
