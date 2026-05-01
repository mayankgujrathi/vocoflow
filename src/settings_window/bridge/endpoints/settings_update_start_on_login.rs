use serde::Deserialize;
use tracing::{error, info};

use crate::autostart;
use crate::decode_payload;
use crate::settings;
use crate::settings_window::bridge::lib::{
  BridgeHttpResponse, BridgeRequest, ResolvedRoute, make_error,
  success_response,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SettingsUpdateStartOnLoginRequest {
  start_on_login: bool,
}

#[derive(Debug, serde::Serialize)]
struct SettingsUpdateStartOnLoginResponse {
  settings: settings::AppSettings,
}

pub fn handle(
  req: &BridgeRequest,
  route: &ResolvedRoute,
) -> BridgeHttpResponse {
  info!(
    request_id = ?req.request_id,
    route = %route.route_kind,
    "settings update start_on_login request"
  );
  let payload: SettingsUpdateStartOnLoginRequest = match decode_payload!(
    req,
    SettingsUpdateStartOnLoginRequest,
    &route.route_kind
  ) {
    Ok(payload) => payload,
    Err(response) => return response,
  };

  match settings::update_start_on_login(payload.start_on_login) {
    Ok(updated_settings) => {
      crate::settings_window::bridge::events::emit_settings_changed(
        &updated_settings,
      );
      if let Err(err) = autostart::sync_from_settings() {
        error!(
          request_id = ?req.request_id,
          error = %err,
          "failed to sync autostart after start_on_login update"
        );
      }
      info!(
        request_id = ?req.request_id,
        start_on_login = payload.start_on_login,
        "settings update start_on_login succeeded"
      );
      success_response(
        req.request_id.clone(),
        route,
        SettingsUpdateStartOnLoginResponse {
          settings: updated_settings,
        },
      )
    }
    Err(err) => {
      error!(
        request_id = ?req.request_id,
        error = %err,
        "settings update start_on_login failed"
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
