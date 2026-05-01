use serde::Deserialize;
use tracing::info;

use crate::decode_payload;
use crate::history;
use crate::settings_window::bridge::lib::{
  BridgeHttpResponse, BridgeRequest, ResolvedRoute, success_response,
};

#[derive(Debug, Deserialize)]
#[serde(default)]
struct SettingsGetHistoryRequest {
  page: u32,
  page_size: u32,
}

impl Default for SettingsGetHistoryRequest {
  fn default() -> Self {
    Self {
      page: 1,
      page_size: 20,
    }
  }
}

#[derive(Debug, serde::Serialize)]
struct SettingsGetHistoryResponse {
  history: history::HistoryPage,
}

pub fn handle(
  req: &BridgeRequest,
  route: &ResolvedRoute,
) -> BridgeHttpResponse {
  info!(
    request_id = ?req.request_id,
    route = %route.route_kind,
    "settings get history request"
  );

  let payload: SettingsGetHistoryRequest =
    decode_payload!(req, SettingsGetHistoryRequest, &route.route_kind)
      .unwrap_or_default();

  let _ = history::cleanup_invalid_entries();

  success_response(
    req.request_id.clone(),
    route,
    SettingsGetHistoryResponse {
      history: history::list_page(payload.page, payload.page_size),
    },
  )
}
