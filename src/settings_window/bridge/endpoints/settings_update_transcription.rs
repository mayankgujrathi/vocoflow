use serde::Deserialize;
use tracing::{debug, error, info};

use crate::decode_payload;
use crate::settings;
use crate::settings_window::bridge::lib::{
  BridgeHttpResponse, BridgeRequest, ResolvedRoute, make_error,
  success_response,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SettingsUpdateTranscriptionRequest {
  transcription: settings::TranscriptionSettings,
}

#[derive(Debug, serde::Serialize)]
struct SettingsUpdateTranscriptionResponse {
  settings: settings::AppSettings,
}

pub fn handle(
  req: &BridgeRequest,
  route: &ResolvedRoute,
) -> BridgeHttpResponse {
  info!(
    request_id = ?req.request_id,
    route = %route.route_kind,
    "settings update transcription request"
  );
  let payload: SettingsUpdateTranscriptionRequest = match decode_payload!(
    req,
    SettingsUpdateTranscriptionRequest,
    &route.route_kind
  ) {
    Ok(payload) => payload,
    Err(response) => return response,
  };

  match settings::update_transcription(payload.transcription) {
    Ok(updated_settings) => {
      crate::settings_window::bridge::events::emit_settings_changed(
        &updated_settings,
      );
      debug!(
        request_id = ?req.request_id,
        model_cache_ttl_secs = updated_settings.transcription.model_cache_ttl_secs,
        llm_base_url = %updated_settings.transcription.llm_base_url,
        llm_model_name = %updated_settings.transcription.llm_model_name,
        "settings transcription updated"
      );
      info!(
        request_id = ?req.request_id,
        "settings update transcription succeeded"
      );
      success_response(
        req.request_id.clone(),
        route,
        SettingsUpdateTranscriptionResponse {
          settings: updated_settings,
        },
      )
    }
    Err(err) => {
      error!(
        request_id = ?req.request_id,
        error = %err,
        "settings update transcription failed"
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
