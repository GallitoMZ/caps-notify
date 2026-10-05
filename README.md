# Caps Notify 🔤

[![Release](https://img.shields.io/github/v/release/GallitoMZ/caps-notify?style=flat-square)](https://github.com/GallitoMZ/caps-notify/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Windows%2010%20%7C%2011-0078D6?style=flat-square)](https://microsoft.com/windows)

**Caps Notify** is a lightweight (< 500 KB), open-source Windows native system tray utility. It provides instant on-screen HUD and audio feedback for **Caps Lock**, **Num Lock**, and **Scroll Lock** keys with event-driven zero polling CPU usage and under 2 MB idle RAM.

---

## 🌟 Key Features

- **Modern Glass HUD & Lenovo Classic OSD**:
  - **Caps Notify Modern** *(Default)*: A sleek, unique obsidian glass HUD with neon glowing jewel accents and dynamic `● ON` / `○ OFF` status capsules.
  - **Lenovo Classic**: Pixel-perfect reproduction of the classic Lenovo OSD (Keycap frame with internal dish arc, pure white `#FFFFFF`, `ABC` / `abc`, and diagonal slash).
  - True per-pixel anti-aliasing rendered directly onto Desktop Window Manager (DWM) via `UpdateLayeredWindow`.
  - Zero lag, 100% reactive: rapidly tapping the lock key updates the state in real-time.
- **9 Screen Positions**:
  - Top Center *(Default)*, Top Right, Top Left, Center, Bottom Center, Bottom Right, Bottom Left, Center Right, Center Left.
- **Synthesized Audio Chimes**:
  - Pure in-memory 16-bit PCM procedural audio (zero external audio files or third-party runtimes).
  - **Modern Chime (Harmonic)**: Gentle crystal harmonic chime (880 Hz ON / 587 Hz OFF).
  - **Mechanical Click**: Subtle tactile key switch click (25 ms).
  - **Windows Default**: Standard system notification sound.
- **Native Settings Window**:
  - Styled with Windows 11 / 10 Common Controls v6 (modern visual styles).
  - Application icon embedded in title bar and taskbar.
  - Spacious 3-card real-time status monitor (`Caps Lock`, `Num Lock`, `Scroll Lock`).
  - Bilingual interface (English default, with Español option).
  - Opens on the **first run only**; subsequent boots start silently into the system tray without interruption.
  - Live **Test HUD** and **Test Sound** preview buttons.
- **Ultra-Efficient Architecture**:
  - Direct Win32 API (`windows` crate 0.58).
  - Low-level keyboard hook (`WH_KEYBOARD_LL`) with asynchronous message dispatch.
  - 0.0% idle CPU and ~1.5 MB private memory.
  - Auto-recovery on `explorer.exe` restart (`TaskbarCreated`).

---

## 📊 Performance Benchmark

| Metric | Caps Notify (Rust) | TrayStatus Free | Lenovo Hotkeys |
|---|---|---|---|
| **Binary Size** | **~478 KB (0.47 MB)** | ~18 MB | ~65 MB |
| **Idle RAM (Private)** | **~1.55 MB** | 45 MB | 70 MB |
| **Idle CPU** | **0.0%** (Event-driven) | ~0.1% - 0.5% | ~0.2% - 1.0% |
| **Runtime Dependencies** | **None** (Native Win32) | .NET 8 / WPF | UWP / Electron |
| **Startup Time** | **< 15 ms** | ~850 ms | ~1500 ms |

---

## 🚀 Build and Run

### Build Optimized Release
```powershell
cargo build --release
```
The standalone single executable will be located at:
```
target\release\caps-notify.exe
```

---

## ⚙️ Configuration (`config.toml`)

Stored at `%APPDATA%\caps-notify\config.toml`:

```toml
# Language: "en" (default) or "es"
language = "en"

# Lock keys to monitor
watch_caps = true
watch_num = true
watch_scroll = false

# HUD Overlay options
overlay_enabled = true
# Themes: "CapsNotifyModern" (Default), "LenovoClassic"
overlay_theme = "CapsNotifyModern"
# Positions: "TopCenter", "TopLeft", "TopRight", "CenterLeft", "Center", "CenterRight", "BottomLeft", "BottomCenter", "BottomRight"
overlay_position = "TopCenter"
# Sizes: "Small" (108px), "Medium" (130px), "Large" (156px)
overlay_size = "Medium"
overlay_duration_ms = 850

# Audio options
sound_enabled = false
# Themes: "ModernChime", "KeyClick", "WindowsDefault"
sound_theme = "ModernChime"

# Windows Toast Notifications
toast_enabled = false

# System options
autostart = false
first_run = false
```

---

## 📄 License

Distributed under the **MIT** License. See [`LICENSE`](LICENSE) for details.
