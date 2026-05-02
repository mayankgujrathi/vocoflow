# Vocoflow

Turn your voice into ready-to-paste text in seconds with a fast, privacy-friendly desktop dictation app built in Rust.

> ⚠️ **Hobby project note:** Vocoflow is currently a hobby project. To keep shipping features quickly, official code-signing/notarization is not fully set up yet.

## Quick Start

## Docs Map

| Goal | Document |
| --- | --- |
| Install and everyday use | [README](README.md) |
| Build, packaging, CI, release, signing | [Build and Release](docs/BUILD_AND_RELEASE.md) |
| Runtime settings, logs, traces | [Settings and Logging](docs/SETTINGS_AND_LOGGING.md) |
| Optional LLM reformatting setup | [LLM Setup and Usage](docs/LLM_SETUP_AND_USAGE.md) |
| Internal architecture and module layout | [Architecture](docs/ARCHITECTURE.md) |

### Requirements

- A working microphone (uses system default mic).

### Install from GitHub Releases (recommended)

1. Open the GitHub repo’s **Releases** tab.
2. Download the latest build for your OS:
   - **Windows:** `vocoflow-<version>-windows-installer.exe`
   - **macOS:** `vocoflow-<version>-macos.dmg`
   - **Linux:** `vocoflow-<version>-linux.AppImage`
3. Install/run using the platform steps below.

If your OS shows a security prompt while opening/installing, see [Unsigned Install Guidance](#unsigned-install-guidance-windowsmacoslinux).

### Package managers

#### winget

```powershell
winget install -e --id mayankgujrathi.vocoflow
```

#### brew

Homebrew installs are available for **macOS and Linux**.

Primary (fully-qualified, recommended to avoid naming conflicts):

```bash
brew install mayankgujrathi/tap/vocoflow
```

Alternative:

```bash
brew tap mayankgujrathi/tap
brew install vocoflow
```

Notes:

- macOS release builds are currently **Apple Silicon only**.
- Windows users should install via `winget`.

### Platform Support Matrix

| OS | Architecture | Preferred install channel | Release asset |
| --- | --- | --- | --- |
| Windows | x64 | `winget` | `vocoflow-<version>-windows-installer.exe` |
| macOS | Apple Silicon | `brew` or Releases | `vocoflow-<version>-macos.dmg` |
| Linux | x64 | `brew` or Releases | `vocoflow-<version>-linux.AppImage` |

## Release Trust (End-user)

- Releases are built and validated in CI across Windows/macOS/Linux.
- Artifacts include integrity/safety checks (like checksum generation and automated security scans) to reduce risk.
- Full technical details are documented in [Build and Release](docs/BUILD_AND_RELEASE.md).

Note: These checks improve trust, but no software distribution can promise zero risk.

### Build from source

```bash
cargo build --release
```

### Run from source

```bash
cargo run --release
```

### Use

1. Press ``Ctrl + ` `` (Windows/Linux) or ``Command + ` `` (macOS) to start recording.
2. Speak normally.
3. Press the same hotkey again to stop.
4. Transcribed text is copied and typed into the active text field.

## Unsigned Install Guidance (Windows/macOS/Linux)

Because this is a hobby project and binaries may be unsigned on some platforms, you may see OS security warnings.

Why this happens: full commercial-style code signing/notarization is still being rolled out.

What assurance you still get: release builds go through CI validation and automated security checks before publishing (see [Build and Release](docs/BUILD_AND_RELEASE.md) for details).

### Windows (Unknown Publisher / SmartScreen)

1. Launch installer.
2. If SmartScreen appears, click **More info**.
3. Click **Run anyway**.

### macOS (Gatekeeper)

1. Try opening the app from Finder.
2. If blocked, right-click app → **Open** → confirm.
3. If still blocked, go to **System Settings → Privacy & Security** and click **Open Anyway** for Vocoflow.

### Linux (AppImage)

1. Make the file executable:
   ```bash
   chmod +x vocoflow-<version>-linux.AppImage
   ```
2. Run it:
   ```bash
   ./vocoflow-<version>-linux.AppImage
   ```

Tip: Always download from the official GitHub Releases page.

### Optional: Verify Release Checksums

You can verify downloaded artifacts with published `.sha256` files from the same release.

- **Windows (PowerShell):**
  ```powershell
  certutil -hashfile .\vocoflow-<version>-windows-installer.exe SHA256
  ```
- **macOS:**
  ```bash
  shasum -a 256 vocoflow-<version>-macos.dmg
  ```
- **Linux:**
  ```bash
  sha256sum vocoflow-<version>-linux.AppImage
  ```

Compare the resulting hash with the value in the corresponding `.sha256` release file.

## First Launch: What to Expect

- The app may take a little longer the first time while it prepares runtime files.
- A `settings.json` file and log files are created automatically.
- You might not see a main window on startup — Vocoflow can run in the background. Check the system tray/menu bar icon to confirm it’s running and open Settings.
- On first launch, you may briefly see a small blue progress pill. That indicates model files are being downloaded for transcription.
- For model source and license details, see [Attribution](docs/LICENSES.md#attribution).
- Depending on your OS, you may be asked to allow microphone/accessibility/security permissions.
- After setup, start dictation with the hotkey and use normally.

## Troubleshooting (Quick)

- **Windows webview issues**: make sure WebView2 runtime is installed.
- **Linux webview issues**: install WebKitGTK stack (`webkit2gtk`, `javascriptcoregtk`, `libsoup3`).
- **macOS WebKit link issues**: build with `RUSTFLAGS="-l framework=WebKit"` when needed.

For full troubleshooting and platform-specific setup details, see the docs below.

## LLM Integration (Optional, OpenAI-compatible)

Vocoflow includes optional LLM-based transcript post-processing for cleaner, context-aware output.

### Why use it (advantages)

- Produces cleaner punctuation/casing automatically.
- Improves readability and polish for notes, docs, chats, and emails.
- Adapts style to the **currently focused app context** (window/app metadata).
- Can move from light cleanup to fully rewritten polished output depending on selected level.

### API compatibility

- Uses an **OpenAI-compatible API** endpoint (`llm_base_url`).
- Works with providers/local gateways that expose OpenAI-style chat completion APIs.

### Default behavior

- By default, LLM reformatting is **not enabled** (`reformatting level = none`).
- In `none` mode, model calls are generally bypassed.

## Settings & Logging

To keep this README focused on install/use, detailed settings and logging behavior has moved to:

- [Settings and Logging](docs/SETTINGS_AND_LOGGING.md)

## Documentation

- [Architecture](docs/ARCHITECTURE.md)
- [Settings and Logging](docs/SETTINGS_AND_LOGGING.md)
- [Build and Release](docs/BUILD_AND_RELEASE.md)
- [LLM Setup and Usage](docs/LLM_SETUP_AND_USAGE.md)
- [Licensing and Acknowledgments](docs/LICENSES.md)

## License

MIT — see [LICENSE](LICENSE).
