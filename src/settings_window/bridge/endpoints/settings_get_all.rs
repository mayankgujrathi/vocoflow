use tracing::{debug, info};

use crate::autostart;
use crate::logging;
use crate::runtime_flash;
use crate::settings;
use crate::settings_window::bridge::lib::{
  BridgeHttpResponse, BridgeRequest, ResolvedRoute, success_response,
};

#[derive(Debug, serde::Serialize)]
struct SettingsGetAllResponse {
  settings: settings::AppSettings,
  logs_dir: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  flash: Option<runtime_flash::SettingsFlashPayload>,
}

pub fn handle(
  req: &BridgeRequest,
  route: &ResolvedRoute,
) -> BridgeHttpResponse {
  info!(
    request_id = ?req.request_id,
    route = %route.route_kind,
    "settings get-all request"
  );
  settings::refresh_from_disk_best_effort("settings_get_all");
  let _ = autostart::sync_settings_from_system();
  let current = settings::current();
  let logs_dir = logging::logs_dir_path().display().to_string();
  let flash = runtime_flash::take_for_settings_flash().ok().flatten();
  crate::settings_window::bridge::events::emit_settings_changed(&current);
  if let Some(flash_payload) = &flash {
    crate::settings_window::bridge::events::emit_settings_flash(flash_payload);
  }
  debug!(
    request_id = ?req.request_id,
    start_on_login = current.start_on_login,
    "settings get-all response payload prepared"
  );
  success_response(
    req.request_id.clone(),
    route,
    SettingsGetAllResponse {
      settings: current,
      logs_dir,
      flash,
    },
  )
}
