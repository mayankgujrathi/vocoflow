pub mod config;
pub mod endpoints;
pub mod events;
pub mod lib;

use tracing::{debug, info, warn};
use wry::WebView;

use self::lib::{
  BridgeHttpResponse, BridgeRequest, IpcRequest, find_route, json_decode_error,
  missing_route_error, normalize_endpoint, supported_routes_text,
  unknown_route_error,
};

fn dispatch_request(req: BridgeRequest) -> BridgeHttpResponse {
  let method = match req.method.as_deref() {
    Some(method) => method.to_ascii_uppercase(),
    None => return missing_route_error(req.request_id),
  };
  let endpoint = match req.endpoint.as_deref() {
    Some(endpoint) => normalize_endpoint(endpoint),
    None => return missing_route_error(req.request_id),
  };
  let route_kind = format!("{} {}", method, endpoint);

  info!(
    method = %method,
    endpoint = %endpoint,
    route_kind = %route_kind,
    request_id = ?req.request_id,
    "settings IPC route resolved"
  );

  match find_route(config::ROUTES, &method, &endpoint) {
    Some(route_def) => {
      let route = self::lib::ResolvedRoute {
        endpoint: endpoint.clone(),
        route_kind,
      };
      (route_def.handler)(&req, &route)
    }
    None => unknown_route_error(
      &method,
      &endpoint,
      req.request_id,
      &supported_routes_text(config::ROUTES),
    ),
  }
}

pub fn handle_ipc(request: IpcRequest) -> BridgeHttpResponse {
  handle_ipc_message(request.body())
}

pub fn handle_ipc_message(payload: &str) -> BridgeHttpResponse {
  let body_size = payload.len();
  info!(
    channel = "wry_ipc",
    body_size, "settings IPC request received"
  );

  match serde_json::from_str::<BridgeRequest>(payload) {
    Ok(req) => {
      info!(
        channel = "wry_ipc",
        method = ?req.method,
        endpoint = ?req.endpoint,
        request_id = ?req.request_id,
        "settings IPC decoded request"
      );
      debug!(channel = "wry_ipc", payload = %req.payload, "settings IPC payload");
      let response = dispatch_request(req);
      info!(
        channel = "wry_ipc",
        response_status = response.status,
        response_size = response.body.len(),
        "settings IPC response ready"
      );
      response
    }
    Err(e) => {
      let response = json_decode_error(payload, &e.to_string());
      warn!(
        channel = "wry_ipc",
        error = %e,
        body_size,
        response_status = response.status,
        "failed to parse settings IPC message"
      );
      response
    }
  }
}

pub fn handle_bridge_request(raw_body: &str) -> BridgeHttpResponse {
  let body_size = raw_body.len();
  info!(
    channel = "custom_protocol",
    body_size, "settings bridge HTTP IPC request received"
  );

  match serde_json::from_str::<BridgeRequest>(raw_body) {
    Ok(req) => {
      info!(
        channel = "custom_protocol",
        method = ?req.method,
        endpoint = ?req.endpoint,
        request_id = ?req.request_id,
        "settings bridge request decoded"
      );
      debug!(channel = "custom_protocol", payload = %req.payload, "settings bridge payload");

      let response = dispatch_request(req);
      info!(
        channel = "custom_protocol",
        response_status = response.status,
        response_size = response.body.len(),
        "settings bridge response ready"
      );
      response
    }
    Err(e) => {
      let response = json_decode_error(raw_body, &e.to_string());
      warn!(
        channel = "custom_protocol",
        error = %e,
        body_size,
        response_status = response.status,
        "failed to parse settings bridge request"
      );
      response
    }
  }
}

/// Future-facing helper for Rust -> JS communication.
pub fn eval_js(webview: &WebView, script: &str) {
  if let Err(e) = webview.evaluate_script(script) {
    warn!(error = %e, "failed to evaluate script in settings webview");
  }
}
