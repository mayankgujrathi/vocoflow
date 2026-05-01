use serde::Deserialize;
use tracing::{error, info};

use crate::decode_payload;
use crate::history;
use crate::runtime_flash;
use crate::settings_window::bridge::lib::{
  BridgeHttpResponse, BridgeRequest, ResolvedRoute, make_error,
  success_response,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SettingsDeleteHistoryEntryRequest {
  id: String,
}

#[derive(Debug, serde::Serialize)]
struct SettingsDeleteHistoryEntryResponse {
  deleted: bool,
}

pub fn handle(
  req: &BridgeRequest,
  route: &ResolvedRoute,
) -> BridgeHttpResponse {
  info!(
    request_id = ?req.request_id,
    route = %route.route_kind,
    "settings delete history entry request"
  );

  let payload: SettingsDeleteHistoryEntryRequest = match decode_payload!(
    req,
    SettingsDeleteHistoryEntryRequest,
    &route.route_kind
  ) {
    Ok(payload) => payload,
    Err(response) => return response,
  };

  match history::delete_entry(payload.id.as_str()) {
    Ok(deleted) => {
      if deleted && let Err(err) = runtime_flash::record_history_changed() {
        error!(
          request_id = ?req.request_id,
          error = %err,
          "failed to write history-changed flash payload after delete"
        );
      }
      success_response(
        req.request_id.clone(),
        route,
        SettingsDeleteHistoryEntryResponse { deleted },
      )
    }
    Err(err) => {
      error!(
        request_id = ?req.request_id,
        error = %err,
        "settings delete history entry failed"
      );
      let error = make_error(
        "HISTORY_DELETE_FAILED",
        format!("Failed to delete history entry: {err}"),
        None,
        None,
        Some(err),
      );
      let body = serde_json::json!({
        "request_id": req.request_id.clone(),
        "ok": false,
        "kind": "error.history_delete_failed",
        "payload": {},
        "error": error,
      })
      .to_string();

      BridgeHttpResponse { status: 500, body }
    }
  }
}
