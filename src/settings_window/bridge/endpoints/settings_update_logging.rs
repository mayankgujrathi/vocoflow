use serde::Deserialize;
use tracing::{debug, error, info};

use crate::decode_payload;
use crate::logging;
use crate::settings;
use crate::settings_window::bridge::lib::{
  BridgeHttpResponse, BridgeRequest, ResolvedRoute, make_error,
  success_response,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SettingsUpdateLoggingRequest {
  logging: settings::LoggingSettings,
}

#[derive(Debug, serde::Serialize)]
struct SettingsUpdateLoggingResponse {
  settings: settings::AppSettings,
}

pub fn handle(
  req: &BridgeRequest,
  route: &ResolvedRoute,
) -> BridgeHttpResponse {
  info!(
    request_id = ?req.request_id,
    route = %route.route_kind,
    "settings update logging request"
  );
  let payload: SettingsUpdateLoggingRequest =
    match decode_payload!(req, SettingsUpdateLoggingRequest, &route.route_kind)
    {
      Ok(payload) => payload,
      Err(response) => return response,
    };

  match settings::update_logging(payload.logging) {
    Ok(updated_settings) => {
      crate::settings_window::bridge::events::emit_settings_changed(
        &updated_settings,
      );
      debug!(
        request_id = ?req.request_id,
        app_log_max_lines = updated_settings.logging.app_log_max_lines,
        trace_file_limit = updated_settings.logging.trace_file_limit,
        enable_debug_logs = updated_settings.logging.enable_debug_logs,
        "applying runtime logging settings"
      );
      logging::apply_runtime_logging_settings();
      info!(
        request_id = ?req.request_id,
        "settings update logging succeeded"
      );
      success_response(
        req.request_id.clone(),
        route,
        SettingsUpdateLoggingResponse {
          settings: updated_settings,
        },
      )
    }
    Err(err) => {
      error!(
        request_id = ?req.request_id,
        error = %err,
        "settings update logging failed"
      );
      let error = make_error(
        "SETTINGS_UPDATE_FAILED",
        format!("Failed to update settings: {err}"),
        None,
        None,
        Some(err),
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
