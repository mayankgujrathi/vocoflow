# Build and Release

This page contains the deeper build, frontend, packaging, and signing notes.

## Install from GitHub Releases

Download the latest release asset matching your OS:

- **Windows:** `vocoflow-<version>-windows-installer.exe`
- **Linux:** `vocoflow-<version>-linux.AppImage`
- **macOS:** `vocoflow-<version>-macos.dmg`

Quick checks after download:

- Verify filename/version match the release tag.
- Ensure you downloaded from the official repository release page.
- On Linux, set AppImage executable permission before running.

## Build

```bash
cargo build --release
```

## Settings Window Frontend (Bun + React + TypeScript)

The settings UI is authored in `ui/settings-window` and compiled into `resources/settings_window` for Wry.

### Build + Sync UI Artifacts

```bash
bun run --cwd ui/settings-window build:sync
```

### Frontend Dev Server

```bash
bun install --cwd ui/settings-window
bun run --cwd ui/settings-window dev
```

Generated files in `resources/settings_window` are build artifacts and typically not tracked in git.

## Platform Notes

### Windows

- WebView2 runtime is required.
- Windows icon embedding uses `winres` in `build.rs` with `assets/activity.ico`.

### Linux

- Requires WebKitGTK stack.
- Typical packages include:
  - `libwebkit2gtk-4.1-dev`
  - `libjavascriptcoregtk-4.1-dev`
  - `libsoup-3.0-dev`
  - `inotify-tools` (for settings flash file-watch propagation)

### macOS

- WebKit is native.
- If needed for specific build contexts:

```bash
RUSTFLAGS="-l framework=WebKit" cargo build --target=<mac-target>
```

## Packaging Outputs (CI/Release)

- **Windows:** `vocoflow-<version>-windows-installer.exe`
- **Linux:** `vocoflow-<version>-linux.AppImage`
- **macOS:** `vocoflow-<version>-macos.dmg`

## Windows Installer Silent Mode

Windows NSIS installer supports quiet install/uninstall for WinGet checks:

- Install silently: `vocoflow-<version>-windows-installer.exe /S`
- Uninstall silently: `"%LOCALAPPDATA%\Programs\Vocoflow\Uninstall.exe" /S`

CI validates silent install behavior during workflow runs.

## Security Scans in CI/CD + Release

Release workflows include baseline trust checks before artifacts are published:

- **Multi-OS CI validation** across Windows, macOS, and Linux build/test jobs.
- **Windows installer silent-mode (`/S`) validation** for package manager compatibility.

- **Windows Defender** scan on Windows installer artifacts.
- **Trivy** filesystem supply-chain scan (high/critical fails CI).
- **SHA256 checksums** generated for release artifacts.
- **Optional VirusTotal** upload scan when `VIRUSTOTAL_API_KEY` is set.

## Runtime Smoke + Health Checks in Release Pipeline

Release workflow validates packaged startup/runtime behavior on all OSes:

- **Windows:** existing silent install smoke test (`/S`) remains unchanged; plus `vocoflow.exe --health-check` on installed binary.
- **Linux:** AppImage startup probe (tray/headless-safe) + `--health-check`.
- **macOS:** DMG app startup probe (tray/headless-safe) + `--health-check` on packaged app.

`--health-check` is a lightweight, non-interactive startup validation mode intended for CI/package-manager verification.

Additional dependency hardening checks are enforced for brew-oriented artifacts:

- **Linux AppImage:** CI extracts the AppImage and fails if `ldd` reports unresolved (`not found`) runtime libraries.
- **macOS app bundle:** CI inspects `otool -L` output and fails on disallowed non-system dependency paths.

## WinGet Publish Automation

On every newly pushed tag, release workflow runs a WinGet publish job that submits/updates a PR to `microsoft/winget-pkgs`.

- Package id: `mayankgujrathi.vocoflow`
- Source installer: GitHub Release Windows installer asset

Required secret:

- `WINGET_TOKEN` (token used by `wingetcreate --submit`)

## Homebrew Publish Automation (macOS + Linux)

On every newly pushed tag, release workflow also updates `mayankgujrathi/homebrew-tap` directly.

- Tap formula path: `Formula/vocoflow.rb`
- Uses release assets:
  - macOS: `vocoflow-<version>-macos.tar.gz` (contains `Vocoflow.app`)
  - Linux: `vocoflow-<version>-linux.AppImage`
- Uses checksums from release `.sha256` files

Install commands exposed to users:

- Primary (fully-qualified): `brew install mayankgujrathi/tap/vocoflow`
- Alternative:
  - `brew tap mayankgujrathi/tap`
  - `brew install vocoflow`

Required secret:

- `WINGET_TOKEN` (classic PAT with `public_repo`, reused for tap direct-push)

Recommended optional secret:

- `VIRUSTOTAL_API_KEY`

## Signing Status

Vocoflow is actively developed as a hobby project. Some release artifacts may not be fully signed/notarized yet.

If your OS warns about publisher identity, follow the platform guidance in the main [README](../README.md#unsigned-install-guidance-windowsmacoslinux).

## Optional Signing / Notarization

Release workflows can use secrets to perform platform signing/notarization when configured.

- Windows signing secrets: `WIN_CERT_PFX_BASE64`, `WIN_CERT_PASSWORD`
- macOS signing/notarization secrets: `MACOS_CERT_P12_BASE64`, `MACOS_CERT_PASSWORD`, `KEYCHAIN_PASSWORD`, `APPLE_ID`, `APPLE_APP_SPECIFIC_PASSWORD`, `APPLE_TEAM_ID`
- Linux authenticity secrets: `LINUX_GPG_PRIVATE_KEY`, `LINUX_GPG_PASSPHRASE`

## Related Docs

- [Architecture](ARCHITECTURE.md)
- [Settings and Logging](SETTINGS_AND_LOGGING.md)
- [Licensing and Acknowledgments](LICENSES.md)
