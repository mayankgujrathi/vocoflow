#![cfg_attr(
  all(target_os = "windows", not(debug_assertions)),
  windows_subsystem = "windows"
)]

mod app;
mod audio;
mod autostart;
mod history;
mod hotkey;
mod llm;
mod logging;
mod runtime_flash;
mod settings;
mod settings_window;
mod tray;

use std::io::Write;
use std::sync::{
  Arc,
  atomic::{AtomicBool, Ordering},
};

use eframe::egui;
use single_instance::SingleInstance;
use tracing::{debug, error, info, warn};

use crate::hotkey::{HotkeyMatcher, Modifiers, TriggerInput};

const HEALTH_CHECK_ARG: &str = "--health-check";

fn detect_resource_root() -> Option<std::path::PathBuf> {
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

  roots.push(std::path::PathBuf::from("."));

  roots.into_iter().find(|root| {
    root
      .join("resources")
      .join("settings_window")
      .join("index.html")
      .exists()
  })
}

fn run_health_check() -> Result<(), String> {
  let exe = std::env::current_exe()
    .map_err(|e| format!("unable to resolve current executable path: {e}"))?;
  if !exe.exists() {
    return Err(format!("executable path does not exist: {}", exe.display()));
  }

  let icon_bytes = include_bytes!("../assets/activity.png");
  image::load_from_memory(icon_bytes)
    .map_err(|e| format!("failed to decode embedded tray icon asset: {e}"))?;

  let resource_root = detect_resource_root().ok_or_else(|| {
    "unable to locate resources/settings_window/index.html from runtime roots"
      .to_string()
  })?;

  let settings_index = resource_root
    .join("resources")
    .join("settings_window")
    .join("index.html");
  if !settings_index.exists() {
    return Err(format!(
      "settings window entrypoint missing: {}",
      settings_index.display()
    ));
  }

  println!(
    "health-check ok: exe={}, resources_root={}",
    exe.display(),
    resource_root.display()
  );
  Ok(())
}

fn main() -> eframe::Result<()> {
  if std::env::args().any(|arg| arg == HEALTH_CHECK_ARG) {
    if let Err(e) = run_health_check() {
      let _ = std::io::stderr()
        .write_all(format!("health-check failed: {e}\n").as_bytes());
      std::process::exit(1);
    }
    return Ok(());
  }

  settings::initialize();
  if let Err(e) = logging::init_logging() {
    // Fallback path before logger is available.
    let _ = std::io::stderr()
      .write_all(format!("Failed to initialize logging: {e}\n").as_bytes());
  }
  info!("application startup initiated");

  if settings_window::should_run_as_settings_process() {
    info!("running in settings-window process mode");
    if let Err(e) = settings_window::run_settings_process() {
      error!(error = %e, "settings-window process failed");
    }
    return Ok(());
  }

  if let Err(e) = autostart::sync_settings_from_system() {
    warn!(error = %e, "failed to sync settings from system autostart state");
  }

  if let Err(e) = autostart::sync_from_settings() {
    warn!(error = %e, "failed to sync autostart from settings");
  }

  // Prevent launching multiple app instances.
  let instance = SingleInstance::new("vocoflow-single-instance")
    .expect("Failed to create app instance lock");
  if !instance.is_single() {
    warn!("vocoflow is already running; exiting duplicate instance");
    return Ok(());
  }

  // Shared exit flag
  let should_exit = Arc::new(AtomicBool::new(false));

  // Set up tray icon on main thread
  let _tray_manager = tray::TrayManager::new(should_exit.clone());
  info!("tray initialized");

  // Spawn background thread for tray event polling
  tray::spawn_poll_thread(should_exit.clone());
  debug!("tray polling thread spawned");

  // Recording state
  let recording_state = audio::RecordingState::new();
  let volume_level = recording_state.volume_level.clone();
  let is_recording = recording_state.is_recording.clone();
  let mic_ready = recording_state.mic_ready.clone();
  let recording_ready = recording_state.recording_ready.clone();

  // Set up global keyboard/mouse listener for configurable start/stop toggle.
  let recording_state_clone = recording_state.clone();
  let should_exit_clone = should_exit.clone();

  let disable_hotkey_listener =
    std::env::var("DICTATION_DISABLE_HOTKEY_LISTENER")
      .ok()
      .map(|v| {
        matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes")
      })
      .unwrap_or(false);

  let _keyboard_handle = std::thread::spawn(move || {
    if disable_hotkey_listener {
      warn!(
        "global keyboard listener disabled by DICTATION_DISABLE_HOTKEY_LISTENER"
      );
      while !should_exit_clone.load(Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_secs(1));
      }
      return;
    }

    debug!("global keyboard listener thread started");
    let mut hotkey_was_pressed = false;
    let mut modifiers = Modifiers::default();
    let mut active_hotkey = settings::current().hotkey;
    let mut matcher = HotkeyMatcher::new(
      active_hotkey.parsed.sequence.clone(),
      active_hotkey.chord_timeout_ms,
    );

    fn map_key_trigger(key: rdev::Key) -> Option<TriggerInput> {
      let token = match key {
        rdev::Key::BackQuote => crate::hotkey::KeyToken::Backquote,
        rdev::Key::Space => crate::hotkey::KeyToken::Space,
        rdev::Key::Tab => crate::hotkey::KeyToken::Tab,
        rdev::Key::Return => crate::hotkey::KeyToken::Enter,
        rdev::Key::Escape => crate::hotkey::KeyToken::Escape,
        rdev::Key::F1 => crate::hotkey::KeyToken::Function(1),
        rdev::Key::F2 => crate::hotkey::KeyToken::Function(2),
        rdev::Key::F3 => crate::hotkey::KeyToken::Function(3),
        rdev::Key::F4 => crate::hotkey::KeyToken::Function(4),
        rdev::Key::F5 => crate::hotkey::KeyToken::Function(5),
        rdev::Key::F6 => crate::hotkey::KeyToken::Function(6),
        rdev::Key::F7 => crate::hotkey::KeyToken::Function(7),
        rdev::Key::F8 => crate::hotkey::KeyToken::Function(8),
        rdev::Key::F9 => crate::hotkey::KeyToken::Function(9),
        rdev::Key::F10 => crate::hotkey::KeyToken::Function(10),
        rdev::Key::F11 => crate::hotkey::KeyToken::Function(11),
        rdev::Key::F12 => crate::hotkey::KeyToken::Function(12),
        k => {
          if let rdev::Key::KeyA = k {
            crate::hotkey::KeyToken::Char('a')
          } else if let rdev::Key::KeyB = k {
            crate::hotkey::KeyToken::Char('b')
          } else if let rdev::Key::KeyC = k {
            crate::hotkey::KeyToken::Char('c')
          } else if let rdev::Key::KeyD = k {
            crate::hotkey::KeyToken::Char('d')
          } else if let rdev::Key::KeyE = k {
            crate::hotkey::KeyToken::Char('e')
          } else if let rdev::Key::KeyF = k {
            crate::hotkey::KeyToken::Char('f')
          } else if let rdev::Key::KeyG = k {
            crate::hotkey::KeyToken::Char('g')
          } else if let rdev::Key::KeyH = k {
            crate::hotkey::KeyToken::Char('h')
          } else if let rdev::Key::KeyI = k {
            crate::hotkey::KeyToken::Char('i')
          } else if let rdev::Key::KeyJ = k {
            crate::hotkey::KeyToken::Char('j')
          } else if let rdev::Key::KeyK = k {
            crate::hotkey::KeyToken::Char('k')
          } else if let rdev::Key::KeyL = k {
            crate::hotkey::KeyToken::Char('l')
          } else if let rdev::Key::KeyM = k {
            crate::hotkey::KeyToken::Char('m')
          } else if let rdev::Key::KeyN = k {
            crate::hotkey::KeyToken::Char('n')
          } else if let rdev::Key::KeyO = k {
            crate::hotkey::KeyToken::Char('o')
          } else if let rdev::Key::KeyP = k {
            crate::hotkey::KeyToken::Char('p')
          } else if let rdev::Key::KeyQ = k {
            crate::hotkey::KeyToken::Char('q')
          } else if let rdev::Key::KeyR = k {
            crate::hotkey::KeyToken::Char('r')
          } else if let rdev::Key::KeyS = k {
            crate::hotkey::KeyToken::Char('s')
          } else if let rdev::Key::KeyT = k {
            crate::hotkey::KeyToken::Char('t')
          } else if let rdev::Key::KeyU = k {
            crate::hotkey::KeyToken::Char('u')
          } else if let rdev::Key::KeyV = k {
            crate::hotkey::KeyToken::Char('v')
          } else if let rdev::Key::KeyW = k {
            crate::hotkey::KeyToken::Char('w')
          } else if let rdev::Key::KeyX = k {
            crate::hotkey::KeyToken::Char('x')
          } else if let rdev::Key::KeyY = k {
            crate::hotkey::KeyToken::Char('y')
          } else if let rdev::Key::KeyZ = k {
            crate::hotkey::KeyToken::Char('z')
          } else {
            return None;
          }
        }
      };
      Some(TriggerInput::Key(token))
    }

    fn map_mouse_trigger(btn: rdev::Button) -> Option<TriggerInput> {
      let token = match btn {
        rdev::Button::Left => crate::hotkey::MouseButtonToken::Left,
        rdev::Button::Right => crate::hotkey::MouseButtonToken::Right,
        rdev::Button::Middle => crate::hotkey::MouseButtonToken::Middle,
        rdev::Button::Unknown(4) => crate::hotkey::MouseButtonToken::Button4,
        rdev::Button::Unknown(5) => crate::hotkey::MouseButtonToken::Button5,
        _ => return None,
      };
      Some(TriggerInput::Mouse(token))
    }

    fn update_modifier_state(
      modifiers: &mut Modifiers,
      key: rdev::Key,
      pressed: bool,
    ) {
      match key {
        rdev::Key::ControlLeft | rdev::Key::ControlRight => {
          modifiers.ctrl = pressed
        }
        rdev::Key::ShiftLeft | rdev::Key::ShiftRight => {
          modifiers.shift = pressed
        }
        rdev::Key::Alt | rdev::Key::AltGr => modifiers.alt = pressed,
        rdev::Key::MetaLeft | rdev::Key::MetaRight => modifiers.meta = pressed,
        _ => {}
      }
    }

    if let Err(e) = rdev::listen(move |event| {
      // Check for tray exit
      if should_exit_clone.load(Ordering::SeqCst) {
        return;
      }

      let latest_hotkey = settings::current().hotkey;
      if latest_hotkey.parsed.normalized != active_hotkey.parsed.normalized
        || latest_hotkey.chord_timeout_ms != active_hotkey.chord_timeout_ms
      {
        active_hotkey = latest_hotkey;
        matcher = HotkeyMatcher::new(
          active_hotkey.parsed.sequence.clone(),
          active_hotkey.chord_timeout_ms,
        );
      }

      if let rdev::EventType::KeyPress(key) = event.event_type {
        update_modifier_state(&mut modifiers, key, true);
        if !hotkey_was_pressed
          && map_key_trigger(key)
            .map(|trigger| {
              matcher.register_trigger(
                modifiers,
                trigger,
                std::time::Instant::now(),
              )
            })
            .unwrap_or(false)
        {
          hotkey_was_pressed = true;
          info!("recording hotkey trigger received");
          app::wake_ui();

          if !app::is_model_ready() {
            warn!("hotkey ignored because speech model is not ready");
            return;
          }

          // Toggle recording
          if recording_state_clone.is_recording() {
            // Stop recording
            recording_state_clone.set_recording(false);
          } else {
            // Start recording
            recording_state_clone.record();
          }
        }
      } else if let rdev::EventType::KeyRelease(key) = event.event_type {
        update_modifier_state(&mut modifiers, key, false);
        if map_key_trigger(key).is_some() {
          hotkey_was_pressed = false;
        }
      } else if let rdev::EventType::ButtonPress(button) = event.event_type {
        if !hotkey_was_pressed
          && map_mouse_trigger(button)
            .map(|trigger| {
              matcher.register_trigger(
                modifiers,
                trigger,
                std::time::Instant::now(),
              )
            })
            .unwrap_or(false)
        {
          hotkey_was_pressed = true;
          info!("recording hotkey trigger received");
          app::wake_ui();

          if !app::is_model_ready() {
            warn!("hotkey ignored because speech model is not ready");
            return;
          }

          if recording_state_clone.is_recording() {
            recording_state_clone.set_recording(false);
          } else {
            recording_state_clone.record();
          }
        }
      } else if let rdev::EventType::ButtonRelease(button) = event.event_type
        && map_mouse_trigger(button).is_some()
      {
        hotkey_was_pressed = false;
      }
    }) {
      error!(error = ?e, "failed to start global keyboard listener");
    }
  });

  let options = eframe::NativeOptions {
    viewport: egui::ViewportBuilder::default()
      .with_inner_size(app::WINDOW_INNER_SIZE)
      .with_decorations(false)
      .with_transparent(true)
      .with_always_on_top()
      .with_position(egui::pos2(0.0, 0.0))
      .with_taskbar(false)
      .with_active(false)
      .with_visible(false),
    ..Default::default()
  };

  // Keep tray manager alive
  std::mem::forget(_tray_manager);

  // Create VoiceApp with new parameters
  let result = eframe::run_native(
    "Voice Widget",
    options,
    Box::new(move |_cc| {
      Box::new(app::VoiceApp::new(
        volume_level,
        is_recording,
        mic_ready,
        recording_ready,
        should_exit,
      ))
    }),
  );

  logging::enforce_app_log_retention();

  result
}
