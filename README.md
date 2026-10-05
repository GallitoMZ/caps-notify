# Caps Notify 🔤

[![Release](https://img.shields.io/github/v/release/GallitoMZ/caps-notify?style=flat-square)](https://github.com/GallitoMZ/caps-notify/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Windows%2010%20%7C%2011-0078D6?style=flat-square)](https://microsoft.com/windows)

**Caps Notify** es una aplicación Windows nativa, ultraligera (< 500 KB), de código abierto y residente en la bandeja del sistema (*System Tray*). Notifica el estado en tiempo real de **Caps Lock** (Bloq Mayús), **Num Lock** (Bloq Num) y **Scroll Lock** (Bloq Despl) con un diseño inspirado en el clásico OSD de Lenovo, sonido armónico moderno y cero polling de CPU.

---

## 🌟 Características clave

- **HUD Flotante OSD estilo Lenovo**:
  - Activado por defecto.
  - Diseño fiel y elegante inspirado en los indicadores OSD de teclas físicas (*Keycap* con esquina redondeada, hendidura esférica 3D y símbolos limpios).
  - **Caps Lock**: Muestra `ABC` en mayúsculas al activarse y `abc` en minúsculas al desactivarse con la flecha `^`.
  - **Num Lock**: Muestra el LED circular y `123`, con tachado diagonal `\` al desactivarse.
  - **Scroll Lock**: Muestra el indicador `SCR` / `scr` con tachado.
  - **100% dinámico y reactivo**: Si pulsas la tecla rápidamente en ráfaga, el indicador actualiza su estado de forma instantánea sin retrasos ni fotogramas desfasados.
  - Renderizado directo en **Desktop Window Manager (DWM)** con alpha real por píxel (`UpdateLayeredWindow`).
- **9 Posiciones en pantalla**:
  - Arriba Centro, Arriba Izquierda, Arriba Derecha.
  - Centro Izquierda, Centro Total, Centro Derecha.
  - Abajo Izquierda, Abajo Centro, Abajo Derecha.
- **Audio Chime Moderno y Sintetizado**:
  - Cero dependencias ni archivos externos.
  - Generador de audio procedural PCM en memoria:
    - **Chime Moderno**: Un ding armónico cristalino y suave (880 Hz para ON, 587 Hz para OFF), al estilo de interfaces modernas como macOS e iOS.
    - **Clic Mecánico**: Sonido táctil y sutil de switch mecánico (25 ms).
    - **Sonido Windows**: Sonido predeterminado del sistema.
- **Ventana Nativa de Configuración**:
  - Se abre al iniciar la aplicación (configurable) o con doble clic en el icono de la bandeja.
  - Muestra el estado en tiempo real de las teclas de bloqueo.
  - Selector de temas, posiciones, tamaños, duración y botones de prueba en vivo (**👁️ Probar HUD** y **🔊 Probar sonido**).
- **Event-Driven puro (0% CPU en reposo)**:
  - Gancho de teclado de bajo nivel (`WH_KEYBOARD_LL`) con cola asíncrona. Cero bucles en segundo plano.
- **Autoarranque opcional con Windows**:
  - Integrado en `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.

---

## 📊 Comparativa de rendimiento

| Métrica | Caps Notify (Rust) | TrayStatus Free | Lenovo Hotkeys |
|---|---|---|---|
| **Tamaño del binario** | **~458 KB (0.45 MB)** | ~18 MB | ~65 MB |
| **RAM en reposo** | **< 4.5 MB** | 45 MB | 70 MB |
| **CPU en reposo** | **0.0%** (Event-driven) | ~0.1% - 0.5% | ~0.2% - 1.0% |
| **Framework requerido** | **Ninguno** (Win32 nativo) | .NET 8 / WPF | UWP / Electron |
| **Tiempo de arranque** | **< 15 ms** | ~850 ms | ~1500 ms |

---

## 🚀 Compilación y ejecución

### Compilación Release optimizada
```powershell
cargo build --release
```
El ejecutable final se genera en:
```
target\release\caps-notify.exe
```

---

## ⚙️ Configuración (`config.toml`)

Ubicado en `%APPDATA%\caps-notify\config.toml`:

```toml
# Teclas de bloqueo a monitorizar
watch_caps = true
watch_num = true
watch_scroll = false

# HUD Flotante (OSD)
overlay_enabled = true
# Opciones: "TopCenter", "TopLeft", "TopRight", "CenterLeft", "Center", "CenterRight", "BottomLeft", "BottomCenter", "BottomRight"
overlay_position = "TopCenter"
# Opciones: "LenovoKeycap", "AccentColor"
overlay_theme = "LenovoKeycap"
# Opciones: "Small" (104px), "Medium" (128px), "Large" (156px)
overlay_size = "Medium"
overlay_duration_ms = 850

# Sonido
sound_enabled = false
# Opciones: "ModernChime", "KeyClick", "WindowsDefault"
sound_theme = "ModernChime"

# Notificaciones Toast de Windows
toast_enabled = false

# Sistema
autostart = false
show_settings_on_start = true
```

---

## 📄 Licencia

Distribuido bajo la Licencia **MIT**. Consulta [`LICENSE`](LICENSE) para más detalles.
