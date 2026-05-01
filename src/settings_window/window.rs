use std::borrow::Cow;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use notify::{
  Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use single_instance::SingleInstance;
use tracing::{debug, error, info, warn};
use winit::{
  application::ApplicationHandler,
  dpi::LogicalSize,
  event::WindowEvent,
  event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
  window::{Icon, Theme, Window},
};
use wry::{WebView, WebViewBuilder};

use crate::settings_window::{
  SETTINGS_WINDOW_ARG, SETTINGS_WINDOW_HEIGHT, SETTINGS_WINDOW_TITLE,
  SETTINGS_WINDOW_WIDTH, bridge,
};

const BRIDGE_RESPONSE_BUILD_ERROR_BODY: &[u8] =
  b"{\"ok\":false,\"kind\":\"error.response_build\"}";
static SETTINGS_WINDOW_UI_READY: AtomicBool = AtomicBool::new(false);
static SETTINGS_WINDOW_PENDING_SCRIPTS: OnceLock<Mutex<Vec<String>>> =
  OnceLock::new();
const SETTINGS_PRELOAD_DARK_BG_SCRIPT: &str = r#"
(() => {
  const applyPreloadDarkTheme = () => {
    const html = document.documentElement;
    const body = document.body;

    if (html) {
      html.style.backgroundColor = '#0b1020';
      html.style.colorScheme = 'dark';
      html.style.height = '100%';
    }

    if (body) {
      body.style.margin = '0';
      body.style.backgroundColor = '#0b1020';
      body.style.color = '#e2e8f0';
      body.style.minHeight = '100vh';
    }
  };

  applyPreloadDarkTheme();
  window.addEventListener('DOMContentLoaded', applyPreloadDarkTheme, { once: true });
})();
"#;
#[cfg(not(debug_assertions))]
const RELEASE_WEBVIEW_HARDENING_SCRIPT: &str = r#"
(() => {
  window.addEventListener('contextmenu', (event) => {
    event.preventDefault();
  }, { capture: true });

  window.addEventListener('keydown', (event) => {
    const key = (event.key || '').toLowerCase();
    const ctrlOrMeta = event.ctrlKey || event.metaKey;
    const shift = event.shiftKey;

    // F12, Ctrl/Cmd+Shift+I/J/C, Ctrl/Cmd+U
    if (
      key === 'f12' ||
      (ctrlOrMeta && shift && (key === 'i' || key === 'j' || key === 'c')) ||
      (ctrlOrMeta && key === 'u')
    ) {
      event.preventDefault();
      event.stopPropagation();
    }
  }, { capture: true });
})();
"#;
const SETTINGS_NOT_FOUND_HTML: &str = r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Vocoflow Settings</title>
    <style>
      body {
        margin: 0;
        padding: 24px;
        font-family: Inter, Segoe UI, Arial, sans-serif;
        background: #111827;
        color: #e5e7eb;
      }
      .card {
        border: 1px solid #374151;
        border-radius: 10px;
        padding: 16px;
        background: #1f2937;
      }
      code {
        color: #93c5fd;
      }
    </style>
  </head>
  <body>
    <div class="card">
      <h1>Settings resources missing</h1>
      <p>
        Could not load <code>resources/settings_window/index.html</code>.
        Please reinstall the app or contact support.
      </p>
    </div>
  </body>
</html>
"#;

fn build_json_response(
  status: u16,
  body: Vec<u8>,
) -> wry::http::Response<Cow<'static, [u8]>> {
  wry::http::Response::builder()
    .status(status)
    .header("Content-Type", "application/json")
    .header("Access-Control-Allow-Origin", "*")
    .header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
    .header("Access-Control-Allow-Headers", "content-type")
    .body(Cow::Owned(body))
    .unwrap_or_else(|e| {
      warn!(error = %e, "failed to build protocol response; using fallback");
      wry::http::Response::builder()
        .status(500)
        .header("Content-Type", "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        .header("Access-Control-Allow-Headers", "content-type")
        .body(Cow::Borrowed(BRIDGE_RESPONSE_BUILD_ERROR_BODY))
        .expect("fallback response builder should not fail")
    })
}

fn build_bytes_response(
  status: u16,
  content_type: &str,
  body: Vec<u8>,
) -> wry::http::Response<Cow<'static, [u8]>> {
  wry::http::Response::builder()
    .status(status)
    .header("Content-Type", content_type)
    .header("Access-Control-Allow-Origin", "*")
    .header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
    .header("Access-Control-Allow-Headers", "content-type")
    .body(Cow::Owned(body))
    .unwrap_or_else(|e| {
      warn!(error = %e, "failed to build bytes response; using fallback");
      wry::http::Response::builder()
        .status(500)
        .header("Content-Type", "application/json")
        .body(Cow::Borrowed(BRIDGE_RESPONSE_BUILD_ERROR_BODY))
        .expect("fallback response builder should not fail")
    })
}

fn settings_resource_roots() -> Vec<PathBuf> {
  let mut roots = Vec::new();

  let exe_path = match std::env::current_exe() {
    Ok(path) => path,
    Err(_) => {
      roots.push(PathBuf::from("."));
      return roots;
    }
  };

  let exe_dir = match exe_path.parent() {
    Some(dir) => dir,
    None => {
      roots.push(PathBuf::from("."));
      return roots;
    }
  };

  roots.push(exe_dir.to_path_buf());
  if let Some(parent) = exe_dir.parent() {
    roots.push(parent.to_path_buf());
  }
  roots.push(exe_dir.join("..").join("Resources"));
  roots.push(exe_dir.join(".."));
  roots.push(PathBuf::from("."));
  roots
}

fn icon_candidate_paths() -> Vec<PathBuf> {
  let mut roots = Vec::new();

  if let Ok(exe_path) = std::env::current_exe()
    && let Some(exe_dir) = exe_path.parent()
  {
    roots.push(exe_dir.to_path_buf());
    if let Some(parent) = exe_dir.parent() {
      roots.push(parent.to_path_buf());
    }
    roots.push(exe_dir.join("..").join("Resources"));
    roots.push(exe_dir.join(".."));
  }

  roots.push(PathBuf::from("."));

  let mut candidates = Vec::new();
  for root in roots {
    // Production-first: packaged settings icon.
    candidates.push(
      root
        .join("resources")
        .join("settings_window")
        .join("favicon.ico"),
    );

    // Build/development fallbacks.
    candidates.push(root.join("assets").join("activity.png"));
    candidates.push(root.join("resources").join("activity.png"));
  }
  candidates
}

fn load_settings_window_icon() -> Option<Icon> {
  for path in icon_candidate_paths() {
    let bytes = match fs::read(&path) {
      Ok(bytes) => bytes,
      Err(_) => continue,
    };

    let decoded = match image::load_from_memory(&bytes) {
      Ok(decoded) => decoded.into_rgba8(),
      Err(e) => {
        warn!(error = %e, path = %path.display(), "failed to decode settings icon image");
        continue;
      }
    };

    let (width, height) = decoded.dimensions();
    match Icon::from_rgba(decoded.into_raw(), width, height) {
      Ok(icon) => {
        info!(path = %path.display(), width, height, "loaded settings window icon");
        return Some(icon);
      }
      Err(e) => {
        warn!(error = %e, path = %path.display(), "failed to build settings window icon from rgba");
      }
    }
  }

  warn!(
    "no settings window icon file found; window will use platform default icon"
  );
  None
}

fn resolve_settings_resource_path(relative: &str) -> Option<PathBuf> {
  let sanitized = relative.trim_start_matches('/');
  settings_resource_roots()
    .into_iter()
    .map(|root| {
      root
        .join("resources")
        .join("settings_window")
        .join(sanitized)
    })
    .find(|candidate| candidate.is_file())
}

fn content_type_for_path(path: &Path) -> &'static str {
  match path.extension().and_then(|e| e.to_str()) {
    Some("html") => "text/html; charset=utf-8",
    Some("css") => "text/css; charset=utf-8",
    Some("js") => "application/javascript; charset=utf-8",
    Some("json") => "application/json; charset=utf-8",
    Some("svg") => "image/svg+xml",
    Some("png") => "image/png",
    Some("jpg") | Some("jpeg") => "image/jpeg",
    Some("ico") => "image/x-icon",
    _ => "application/octet-stream",
  }
}

fn handle_settings_protocol_request(
  request: &wry::http::Request<Vec<u8>>,
) -> wry::http::Response<Cow<'static, [u8]>> {
  let method = request.method().as_str().to_string();
  let path = request.uri().path().trim_start_matches('/').to_string();
  debug!(%method, %path, "settings custom protocol request received");

  if request.method() == wry::http::Method::OPTIONS {
    return build_json_response(204, Vec::new());
  }

  if path == "ipc" {
    let body = String::from_utf8_lossy(request.body());
    debug!(
      method = %method,
      path = %path,
      request_body_size = body.len(),
      "settings IPC HTTP request accepted"
    );
    let response = bridge::handle_bridge_request(body.as_ref());
    debug!(
      method = %method,
      path = %path,
      response_status = response.status,
      response_body_size = response.body.len(),
      "settings IPC HTTP response ready"
    );
    return build_json_response(response.status, response.body.into_bytes());
  }

  if request.method() != wry::http::Method::GET {
    return build_json_response(
      405,
      b"{\"ok\":false,\"kind\":\"error.method_not_allowed\"}".to_vec(),
    );
  }

  let requested = if path.is_empty() {
    "index.html".to_string()
  } else if let Some(stripped) = path.strip_prefix("settings/") {
    stripped.to_string()
  } else if let Some(stripped) = path.strip_prefix("localhost/settings/") {
    stripped.to_string()
  } else {
    path
  };

  let Some(resource_path) = resolve_settings_resource_path(&requested) else {
    if requested == "index.html" {
      warn!("settings index.html not found; serving fallback inline page");
      return build_bytes_response(
        200,
        "text/html; charset=utf-8",
        SETTINGS_NOT_FOUND_HTML.as_bytes().to_vec(),
      );
    }
    warn!(requested = %requested, "settings resource not found");
    return build_bytes_response(
      404,
      "text/plain; charset=utf-8",
      b"Not Found".to_vec(),
    );
  };

  match fs::read(&resource_path) {
    Ok(bytes) => {
      let content_type = content_type_for_path(&resource_path);
      build_bytes_response(200, content_type, bytes)
    }
    Err(e) => {
      warn!(error = %e, path = %resource_path.display(), "failed to read settings resource file");
      build_bytes_response(
        500,
        "text/plain; charset=utf-8",
        b"Internal Server Error".to_vec(),
      )
    }
  }
}

fn is_settings_flash_event(event: &Event) -> bool {
  if !matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_)) {
    return false;
  }

  event.paths.iter().any(|path| {
    path
      .file_name()
      .and_then(|name| name.to_str())
      .map(|name| name.eq_ignore_ascii_case("settings_flash.json"))
      .unwrap_or(false)
  })
}

fn start_settings_flash_watcher() {
  let watch_dir = crate::settings::data_dir();
  if let Err(e) = fs::create_dir_all(&watch_dir) {
    warn!(error = %e, path = %watch_dir.display(), "failed to create settings data dir for flash watcher");
    return;
  }

  std::thread::spawn(move || {
    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();
    let mut watcher = match RecommendedWatcher::new(
      move |result| {
        let _ = tx.send(result);
      },
      Config::default(),
    ) {
      Ok(watcher) => watcher,
      Err(e) => {
        warn!(error = %e, "failed to create settings flash watcher");
        return;
      }
    };

    if let Err(e) = watcher.watch(&watch_dir, RecursiveMode::NonRecursive) {
      warn!(error = %e, path = %watch_dir.display(), "failed to watch settings data dir for flash file changes");
      return;
    }

    info!(path = %watch_dir.display(), "settings flash watcher started");
    let mut last_emit_at = Instant::now()
      .checked_sub(Duration::from_secs(1))
      .unwrap_or_else(Instant::now);

    loop {
      match rx.recv() {
        Ok(Ok(event)) => {
          if !is_settings_flash_event(&event) {
            continue;
          }

          let now = Instant::now();
          if now.duration_since(last_emit_at) < Duration::from_millis(200) {
            continue;
          }
          last_emit_at = now;

          match crate::runtime_flash::take_for_settings_flash() {
            Ok(Some(flash)) => {
              crate::settings_window::bridge::events::emit_settings_flash(
                &flash,
              );
            }
            Ok(None) => {}
            Err(e) => {
              warn!(error = %e, "failed reading settings flash payload from watcher event");
            }
          }
        }
        Ok(Err(e)) => {
          warn!(error = %e, "settings flash watcher received notify error");
        }
        Err(_) => break,
      }
    }
  });
}

pub fn should_run_as_settings_process() -> bool {
  std::env::args().any(|arg| arg == SETTINGS_WINDOW_ARG)
}

pub fn open_settings_window() {
  match std::env::current_exe() {
    Ok(exe_path) => {
      let mut command = Command::new(exe_path);
      command.arg(SETTINGS_WINDOW_ARG);
      command.stdout(std::process::Stdio::null());
      command.stderr(std::process::Stdio::null());

      if let Err(e) = command.spawn() {
        error!(error = %e, "failed to spawn settings window process");
      }
    }
    Err(e) => {
      error!(error = %e, "failed to resolve current executable path for settings window");
    }
  }
}

pub fn run_settings_process() -> Result<(), String> {
  SETTINGS_WINDOW_UI_READY.store(false, Ordering::Release);
  let instance =
    SingleInstance::new("vocoflow-settings-window-single-instance").map_err(
      |e| format!("failed to create settings process instance lock: {e}"),
    )?;
  if !instance.is_single() {
    info!("settings window process already running; exiting duplicate request");
    return Ok(());
  }

  start_settings_flash_watcher();

  let event_loop = EventLoop::new().map_err(|e| e.to_string())?;
  event_loop.set_control_flow(ControlFlow::Wait);

  let mut app = SettingsWindowApp::default();
  event_loop.run_app(&mut app).map_err(|e| e.to_string())
}

#[derive(Default)]
struct SettingsWindowApp {
  window: Option<Window>,
  webview: Option<WebView>,
  window_shown: bool,
}

pub(crate) fn mark_settings_window_ui_ready() {
  SETTINGS_WINDOW_UI_READY.store(true, Ordering::Release);
}

pub(crate) fn queue_script_eval(script: String) {
  let queue =
    SETTINGS_WINDOW_PENDING_SCRIPTS.get_or_init(|| Mutex::new(Vec::new()));
  if let Ok(mut scripts) = queue.lock() {
    scripts.push(script);
  }
}

fn drain_queued_scripts() -> Vec<String> {
  let queue =
    SETTINGS_WINDOW_PENDING_SCRIPTS.get_or_init(|| Mutex::new(Vec::new()));
  if let Ok(mut scripts) = queue.lock() {
    return std::mem::take(&mut *scripts);
  }
  Vec::new()
}

impl ApplicationHandler for SettingsWindowApp {
  fn resumed(&mut self, event_loop: &ActiveEventLoop) {
    if self.window.is_some() {
      return;
    }

    #[cfg(target_os = "linux")]
    if let Err(e) = gtk::init() {
      warn!(error = %e, "failed to initialize GTK before creating settings webview");
    }

    let mut attributes = Window::default_attributes()
      .with_title(SETTINGS_WINDOW_TITLE)
      .with_inner_size(LogicalSize::new(
        SETTINGS_WINDOW_WIDTH,
        SETTINGS_WINDOW_HEIGHT,
      ))
      .with_theme(Some(Theme::Dark))
      .with_visible(false)
      .with_resizable(false);

    if let Some(icon) = load_settings_window_icon() {
      attributes = attributes.with_window_icon(Some(icon));
    }

    let window = match event_loop.create_window(attributes) {
      Ok(window) => window,
      Err(e) => {
        error!(error = %e, "failed to create settings window");
        event_loop.exit();
        return;
      }
    };

    let mut webview_builder = WebViewBuilder::new()
      .with_custom_protocol("vocoflow".into(), |_webview_id, request| {
        handle_settings_protocol_request(&request)
      })
      .with_url("vocoflow://localhost/settings/index.html")
      .with_initialization_script(SETTINGS_PRELOAD_DARK_BG_SCRIPT)
      .with_ipc_handler(|request| {
        bridge::handle_ipc(request);
      });

    #[cfg(not(debug_assertions))]
    {
      webview_builder = webview_builder
        .with_initialization_script(RELEASE_WEBVIEW_HARDENING_SCRIPT)
        .with_devtools(false);
    }

    #[cfg(debug_assertions)]
    {
      webview_builder = webview_builder.with_devtools(true);
    }

    let webview = match webview_builder.build(&window) {
      Ok(webview) => webview,
      Err(e) => {
        error!(error = %e, "failed to build settings webview");
        event_loop.exit();
        return;
      }
    };

    bridge::eval_js(&webview, "window.__RUST_BRIDGE_READY__ = true;");
    self.webview = Some(webview);
    self.window = Some(window);
  }

  fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
    if let Some(webview) = &self.webview {
      for script in drain_queued_scripts() {
        bridge::eval_js(webview, &script);
      }
    }

    if !self.window_shown
      && SETTINGS_WINDOW_UI_READY.swap(false, Ordering::AcqRel)
      && let Some(window) = &self.window
    {
      window.set_visible(true);
      self.window_shown = true;
    }

    #[cfg(target_os = "linux")]
    while gtk::events_pending() {
      gtk::main_iteration_do(false);
    }
  }

  fn window_event(
    &mut self,
    event_loop: &ActiveEventLoop,
    _window_id: winit::window::WindowId,
    event: WindowEvent,
  ) {
    if matches!(event, WindowEvent::CloseRequested) {
      if let Some(window) = &self.window {
        window.set_visible(false);
      }
      event_loop.exit();
    }
  }
}
