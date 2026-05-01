use serde::Deserialize;
use tracing::{debug, error, info};

use crate::decode_payload;
use crate::history;
use crate::settings;
use crate::settings_window::bridge::lib::{
  BridgeHttpResponse, BridgeRequest, ResolvedRoute, make_error,
  success_response,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SettingsUpdateHistoryRequest {
  history: settings::HistorySettings,
}

#[derive(Debug, serde::Serialize)]
struct SettingsUpdateHistoryResponse {
  settings: settings::AppSettings,
}

pub fn handle(
  req: &BridgeRequest,
  route: &ResolvedRoute,
) -> BridgeHttpResponse {
  info!(
    request_id = ?req.request_id,
    route = %route.route_kind,
    "settings update history request"
  );

  let payload: SettingsUpdateHistoryRequest =
    match decode_payload!(req, SettingsUpdateHistoryRequest, &route.route_kind)
    {
      Ok(payload) => payload,
      Err(response) => return response,
    };

  match settings::update_history(payload.history) {
    Ok(updated_settings) => {
      history::enforce_retention_from_settings();
      crate::settings_window::bridge::events::emit_settings_changed(
        &updated_settings,
      );
      debug!(
        request_id = ?req.request_id,
        retention_max_sessions = updated_settings.history.retention_max_sessions,
        "settings history updated"
      );
      success_response(
        req.request_id.clone(),
        route,
        SettingsUpdateHistoryResponse {
          settings: updated_settings,
        },
      )
    }
    Err(err) => {
      error!(
        request_id = ?req.request_id,
        error = %err,
        "settings update history failed"
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
