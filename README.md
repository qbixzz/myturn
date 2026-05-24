# MyTurn

Windows system tray app that tracks Claude Code's context window usage in real time.

## How it works

1. A Claude Code `Stop` hook writes `~/.claude/myturn-bridge.json` after every turn
2. MyTurn watches that file and updates the tray icon — a color-coded progress bar showing context window fill %
3. Click the tray icon to open a flyout with token counts, model, and session ID

Color bands: green (0–44%) → yellow (45–64%) → orange (65–84%) → red (85–100%)

## Prerequisites

- Windows 10/11
- [Rust](https://rustup.rs/) with MSVC toolchain (`x86_64-pc-windows-msvc`)
- [VS Build Tools 2022](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022) — C++ workload
- Node.js (for `npx @tauri-apps/cli`)
- WebView2 runtime (ships with Windows 11; download for Windows 10)

## Install the Stop hook

Add to `~/.claude/settings.json`:

```json
{
  "hooks": {
    "Stop": [{
      "hooks": [{
        "type": "command",
        "command": "node /path/to/myturn-bridge.js",
        "timeout": 5
      }]
    }]
  }
}
```

Copy `hooks/myturn-bridge.js` to `~/.claude/hooks/` and update the path above.

> **WSL users**: the hook auto-detects WSL and writes the bridge file to your Windows home (`C:\Users\<you>\.claude\`) so the Windows Tauri app can find it.

## Build

```bash
cd src-tauri
cargo build --release           # produces target/release/myturn.exe

# or build the MSI installer
npx @tauri-apps/cli build        # produces target/release/bundle/msi/MyTurn_*.msi
```

## Config

On first run, MyTurn writes `%APPDATA%\dev.myturn.app\config.json` with default color rules. Edit it to customize thresholds or colors; changes take effect on the next turn.

## Project structure

```
dist/               frontend flyout (plain HTML/JS, no bundler)
hooks/
  myturn-bridge.js  Stop hook — copy to ~/.claude/hooks/
hook-tests/         Node.js integration tests for the Stop hook
src-tauri/
  src/
    bridge.rs       bridge file parser
    color_rules.rs  color threshold engine
    config.rs       config loader
    icon.rs         32×32 RGBA tray icon renderer
    watcher.rs      file watcher → tray + emit
    lib.rs          Tauri setup, commands, AppState
  tauri.conf.json
  Cargo.toml
```
