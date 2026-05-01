use serde::Deserialize;
use tracing::{error, info};

use crate::decode_payload;
use crate::logging;
use crate::settings;
use crate::settings_window::bridge::lib::{
  BridgeHttpResponse, BridgeRequest, ResolvedRoute, make_error,
  success_response,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SettingsResetDefaultsRequest {
  scope: String,
}

#[derive(Debug, serde::Serialize)]
struct SettingsResetDefaultsResponse {
  settings: settings::AppSettings,
}

pub fn handle(
  req: &BridgeRequest,
  route: &ResolvedRoute,
) -> BridgeHttpResponse {
  info!(
    request_id = ?req.request_id,
    route = %route.route_kind,
    "settings reset-defaults request"
  );

  let payload: SettingsResetDefaultsRequest =
    match decode_payload!(req, SettingsResetDefaultsRequest, &route.route_kind)
    {
      Ok(payload) => payload,
      Err(response) => return response,
    };

  let result = match payload.scope.as_str() {
    "general" => settings::reset_general_defaults(),
    "logging" => settings::reset_logging_default(),
    "transcription" => settings::reset_transcription_default(),
    "all" => settings::reset_all_defaults(),
    _ => Err(format!("unknown reset scope: {}", payload.scope)),
  };

  match result {
    Ok(updated_settings) => {
      crate::settings_window::bridge::events::emit_settings_changed(
        &updated_settings,
      );
      if matches!(payload.scope.as_str(), "logging" | "all") {
        logging::apply_runtime_logging_settings();
      }

      success_response(
        req.request_id.clone(),
        route,
        SettingsResetDefaultsResponse {
          settings: updated_settings,
        },
      )
    }
    Err(err) => {
      error!(
        request_id = ?req.request_id,
        error = %err,
        scope = %payload.scope,
        "settings reset-defaults failed"
      );
      let error = make_error(
        "SETTINGS_RESET_FAILED",
        format!("Failed to reset settings defaults: {err}"),
        Some("scope"),
        Some("general|logging|transcription|all"),
        Some(payload.scope),
      );
      let body = serde_json::json!({
        "request_id": req.request_id.clone(),
        "ok": false,
        "kind": "error.settings_reset_failed",
        "payload": {},
        "error": error,
      })
      .to_string();

      BridgeHttpResponse { status: 500, body }
    }
  }
}
