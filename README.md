# Caps Notify 🔤

[![Release](https://img.shields.io/github/v/release/GallitoMZ/caps-notify?style=flat-square)](https://github.com/GallitoMZ/caps-notify/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Windows%2010%20%7C%2011-0078D6?style=flat-square)](https://microsoft.com/windows)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange?style=flat-square&logo=rust)](https://www.rust-lang.org/)

**Caps Notify** is an ultra-lightweight (< 500 KB), open-source Windows native system tray indicator and on-screen display (HUD) for **Caps Lock**, **Num Lock**, and **Scroll Lock** keys.

Designed for peak efficiency and aesthetics: **0.0% idle CPU** (event-driven, zero polling), **~1.5 MB RAM**, zero runtime dependencies, and instant reactive response.

---

## 🌟 Key Features

### 🎨 Reimagined On-Screen Display (HUD)
- **4 Distinct Visual Themes**:
  - **`Modern Glass` (Default)**: Reimagined unified obsidian squircle card (`#0D1117`, `alpha = 252` for zero bleed-through) with a subtle luminous teal glow (`#2DD4BF`), clean Segoe UI Semibold header, bold hero typography, and an integrated `[ ● ON ]` / `[ ○ OFF ]` status pill. Rendered with exact continuous 2D Signed Distance Fields (SDF) for flawless anti-aliasing.
  - **`Cyber Minimal`**: Low-profile tactical horizontal capsule (`[icon] ABC/123 [ ON/OFF ]`) for ultra-compact, non-intrusive notification.
  - **`Neumorphic Keycap`**: Tactile 3D physical keyboard keycap with beveled edges and a soft LED indicator.
  - **`Dynamic Island`**: Fluid, pill-shaped status capsule inspired by modern mobile status islands.
- **Intuitive Status Indicators**:
  - **Caps Lock**: Displays **`ABC`** in crisp bold uppercase when ON, and **`abc`** in lowercase when OFF.
  - **Num Lock**: Displays **`123`** in crisp bold when ON, and **`123` with a clean diagonal slash `\`** in coral red (`#EF4444`) when OFF.
  - **Scroll Lock**: Displays **`SCR`** when ON, and **`SCR` with diagonal slash `\`** when OFF.
- **9 Screen Positions**:
  - Top Center *(Default)*, Top Left, Top Right, Center Left, Center, Center Right, Bottom Left, Bottom Center, Bottom Right.
- **Configurable Sizes**:
  - Compact (108 px), Normal (130/144 px), Large (156 px).
- **Smooth Animation & Timing**:
  - Configurable display duration (default 850 ms) with per-pixel DWM alpha blending via `UpdateLayeredWindow`.
  - Zero lag: rapidly toggling the lock key updates the display in real-time.

---

### 🔊 Procedural Audio Chimes
- **100% In-Memory Synthesized Audio**:
  - Generated via procedural 16-bit PCM waveform synthesis (no external `.wav` files or media dependencies).
  - **Modern Chime (Harmonic)**: Crystal harmonic double chime (880 Hz ON / 587 Hz OFF).
  - **Mechanical Click**: Subtle 25 ms tactile click mimicking mechanical key switches.
  - **Windows Default**: Uses the standard Windows system notification sound.

---

### 🖥️ Native Settings GUI & Bilingual Support
- **Windows 11 / 10 Visual Styles**:
  - Styled with Common-Controls 6.0 and native Segoe UI typography.
  - Application icon embedded in the titlebar, taskbar, and alt-tab switcher.
- **Real-Time 3-Card Status Monitor**:
  - Displays live states for Caps Lock, Num Lock, and Scroll Lock with instant synchronization.
- **Bilingual Interface**:
  - Full localization support for **English** (default) and **Español**, selectable in Settings and tray menu.
- **First-Run Experience**:
  - Opens configuration automatically on the **first run only**; subsequent boots start silently in the background into the system tray.
  - When enabled, Windows Autostart passes `--minimized` to ensure silent background startup.
- **Live Preview Controls**:
  - Instant **Test HUD** and **Test Sound** buttons to audition settings before saving.

---

### ⚡ Ultra-Efficient Architecture
- **Zero Polling**: Uses a low-level Win32 keyboard hook (`WH_KEYBOARD_LL`) that dispatches asynchronously via `PostMessageW(main_hwnd, WM_APP_HOOK, ...)`. Does not block typing input or waste CPU cycles.
- **Native Win32**: Built directly on the official `windows` crate (v0.58). No Electron, no WPF, no .NET runtime, and no Visual C++ Redistributable required.
- **Crash Recovery**: Listens for the `TaskbarCreated` window message to automatically restore the tray icon if `explorer.exe` restarts.
- **Memory Footprint**: Consistently stays below **2 MB** private working set.

---

## 📊 Performance Comparison

| Metric | Caps Notify (Rust) | TrayStatus Free | Lenovo Hotkeys |
|---|---|---|---|
| **Binary Size** | **~480 KB (0.47 MB)** | ~18 MB | ~65 MB |
| **Idle RAM (Private)** | **~1.55 MB** | 45 MB | 70 MB |
| **Idle CPU** | **0.0%** (Event-driven) | ~0.1% - 0.5% | ~0.2% - 1.0% |
| **Runtime Dependencies** | **None** (Native Win32) | .NET 8 / WPF | UWP / Electron |
| **Cold Startup Time** | **< 15 ms** | ~850 ms | ~1500 ms |

---

## 🚀 Installation & Usage

### Pre-built Binaries
Download the latest version from [**GitHub Releases**](https://github.com/GallitoMZ/caps-notify/releases):
- **`caps-notify-setup.exe`**: Classic Windows installer with start menu integration, desktop shortcut, optional autostart, and clean uninstaller in Windows Settings.
- **`caps-notify-portable.exe`**: Standalone single-file executable. No installation required; run directly from any folder or USB drive.

#### 💻 System & Architecture Compatibility
- **x86_64 / AMD64 (64-bit)**: Native support for all modern Intel Core and AMD Ryzen systems on Windows 10 and 11.
- **ARM64 (Snapdragon / Surface)**: Fully compatible with Windows 11 on ARM (e.g. Snapdragon X Elite, Surface Pro 11) via the native Prism emulation engine with zero configuration needed.


### Build from Source
**Prerequisites**:
1. [Rust toolchain](https://rustup.rs/) (`stable-x86_64-pc-windows-msvc`)
2. Visual Studio Build Tools 2022 (with "Desktop development with C++" for the MSVC linker)

```powershell
# Clone the repository
git clone https://github.com/GallitoMZ/caps-notify.git
cd caps-notify

# Compile release build
cargo build --release
```

The compiled standalone executable will be located at:
```
target\release\caps-notify.exe
```

---

## ⚙️ Configuration (`config.toml`)

Configuration is automatically stored in `%APPDATA%\caps-notify\config.toml`:

```toml
# Language: "en" (default) or "es"
language = "en"

# Lock keys to monitor
watch_caps = true
watch_num = true
watch_scroll = false

# HUD Overlay options
overlay_enabled = true
# Themes: "CapsNotifyModern" (Default), "CyberMinimal", "NeumorphicKey", "DynamicIsland"
overlay_theme = "CapsNotifyModern"
# Positions: "TopCenter", "TopLeft", "TopRight", "CenterLeft", "Center", "CenterRight", "BottomLeft", "BottomCenter", "BottomRight"
overlay_position = "TopCenter"
# Sizes: "Small" (108 px), "Medium" (130/144 px), "Large" (156 px)
overlay_size = "Medium"
overlay_duration_ms = 850

# Audio options
sound_enabled = false
# Sound Themes: "ModernChime", "KeyClick", "WindowsDefault"
sound_theme = "ModernChime"

# Windows Toast Notifications
toast_enabled = false

# System options
autostart = false
first_run = false
```

---

## ⌨️ System Tray Menu Shortcuts

Right-clicking the tray icon provides instant access to:
- **Settings...**: Opens the configuration GUI.
- **HUD Theme**: Quick-switch between all 4 visual styles.
- **HUD Position**: Quick-switch between all 9 screen positions.
- **Sound Style**: Quick-switch between the 3 procedural sound styles.
- **Floating HUD**: Quick toggle ON/OFF.
- **Sound Chime**: Quick toggle ON/OFF.
- **Autostart on Boot**: Toggle Windows auto-start.
- **Open config.toml**: Opens the raw configuration file in default editor.
- **About Caps Notify**: Shows version and license information.
- **Exit**: Completely closes the application.

---

## 📄 License

Distributed under the **MIT License**. See [`LICENSE`](LICENSE) for details.

Developed with ❤️ by [**GallitoMZ**](https://github.com/GallitoMZ).
