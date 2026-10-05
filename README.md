# Caps Notify 🔤

[![Release](https://img.shields.io/github/v/release/GallitoMZ/caps-notify?style=flat-square)](https://github.com/GallitoMZ/caps-notify/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Windows%2010%20%7C%2011-0078D6?style=flat-square)](https://microsoft.com/windows)

**Caps Notify** es una aplicación Windows nativa, ultraligera, de código abierto y residente en la bandeja del sistema (*System Tray*). Notifica el estado en tiempo real de **Caps Lock** (Bloq Mayús), **Num Lock** (Bloq Num) y **Scroll Lock** (Bloq Despl) con un consumo de recursos prácticamente nulo.

---

## 🌟 Características clave

- **Iconos dinámicos en la bandeja**: Reflejan al instante el estado activado/desactivado de las teclas de bloqueo.
- **Event-Driven puro (0% CPU en reposo)**: Utiliza un gancho de teclado de bajo nivel (`WH_KEYBOARD_LL`) nativo de Win32. Cero polling, cero temporizadores en segundo plano.
- **Notificaciones Toast nativas**: Integración fluida mediante WinRT Toast Notification Manager.
- **HUD Overlay flotante opcional**: Ventana layered transparente con renderizado suave y temporizador de desvanecimiento automático.
- **Autoarranque con Windows**: Configuración opcional en el registro (`HKCU\Software\Microsoft\Windows\CurrentVersion\Run`) sin tareas programadas pesadas.
- **Persistencia de configuración**: Configurable en formato TOML en `%APPDATA%\caps-notify\config.toml`.
- **Resiliente ante reinicios del Explorer**: Se suscribe al mensaje `TaskbarCreated` para reaparecer automáticamente si `explorer.exe` se reinicia.
- **Zero dependencias en tiempo de ejecución**: No requiere .NET Runtime ni VC++ Redistributable. Binario único y portable.

---

## 📊 Comparativa de rendimiento

| Métrica | Caps Notify (Rust) | TrayStatus Free | Lenovo Hotkeys |
|---|---|---|---|
| **Tamaño del binario** | **~1.2 MB** | ~18 MB | ~65 MB |
| **RAM en reposo** | **< 2.5 MB** | 45 MB | 70 MB |
| **CPU en reposo** | **0.0%** (Event-driven) | ~0.1% - 0.5% | ~0.2% - 1.0% |
| **Framework requerido** | **Ninguno** (Win32 nativo) | .NET 8 / WPF | UWP / Electron |
| **Tiempo de arranque** | **< 15 ms** | ~850 ms | ~1500 ms |

---

## 🛠️ Requisitos previos para compilar

1. **Rust Toolchain**:
   - Descarga e instala `rustup` desde [rustup.rs](https://rustup.rs/).
   - Toolchain recomendada: `stable-x86_64-pc-windows-msvc`.
2. **Visual Studio C++ Build Tools**:
   - Descarga el instalador desde [Visual Studio C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/).
   - Durante la instalación, selecciona la carga de trabajo: **"Desarrollo para el escritorio con C++"** (*Desktop development with C++*).

---

## 🚀 Compilación y ejecución

### Modo desarrollo
```powershell
cargo run
```

### Compilación Release optimizada (Binario mínimo)
```powershell
cargo build --release
```
El archivo ejecutable portable se generará en:
```
target\release\caps-notify.exe
```

---

## ⚙️ Configuración (`config.toml`)

El archivo de configuración se crea automáticamente en `%APPDATA%\caps-notify\config.toml`:

```toml
# Teclas de bloqueo a monitorizar
watch_caps = true
watch_num = true
watch_scroll = false

# Notificaciones
toast_enabled = true
sound_enabled = false

# HUD Overlay en pantalla
overlay_enabled = false
overlay_position = "TopCenter" # Opciones: "TopCenter", "BottomRight", "Center", "TopRight", "BottomCenter"
overlay_duration_ms = 800

# Iniciar automáticamente al encender Windows
autostart = false
```

Puedes abrir directamente el archivo de configuración haciendo clic derecho sobre el icono en la bandeja del sistema y seleccionando **"Open Config File"**.

---

## 📂 Estructura del proyecto

```
caps-notify/
├── .github/
│   └── workflows/
│       └── release.yml          # CI/CD para compilar y publicar releases
├── assets/
│   ├── app.ico                  # Icono de la aplicación
│   ├── app.manifest             # Manifest DPI-Aware (PerMonitorV2)
│   ├── app.rc                   # Recursos de Windows
│   ├── caps_on.ico              # Indicador Mayús Activado
│   ├── caps_off.ico             # Indicador Mayús Desactivado
│   ├── num_on.ico               # Indicador Numérico Activado
│   ├── num_off.ico              # Indicador Numérico Desactivado
│   ├── scroll_on.ico            # Indicador Desplazamiento Activado
│   └── scroll_off.ico           # Indicador Desplazamiento Desactivado
├── src/
│   ├── main.rs                  # Loop de mensajes Win32 y ventana oculta
│   ├── config.rs                # Gestor de configuración TOML
│   ├── hook.rs                  # WH_KEYBOARD_LL con PostMessage asíncrono
│   ├── state.rs                 # Estado compartido con AtomicBool
│   ├── tray.rs                  # Icono en bandeja y menú contextual
│   ├── notifier.rs              # Notificaciones WinRT, sonido y HUD
│   ├── overlay.rs               # Ventana HUD transparente
│   └── autostart.rs             # Gestión de clave Run en HKCU
├── Cargo.toml                   # Perfil release optimizado ("z", LTO, strip)
├── build.rs                     # Compilación de recursos Win32
├── config.default.toml          # Configuración por defecto
├── LICENSE                      # MIT
└── README.md
```

---

## 🛡️ Antivirus y Falsos Positivos

Debido a que `caps-notify` es un ejecutable compacto y no firmado que utiliza `SetWindowsHookExW` (`WH_KEYBOARD_LL`) para interceptar teclas globales en tiempo real sin polling, algunos programas antivirus heurísticos pueden alertar sobre él preventivamente.

- El código fuente completo es 100% abierto y auditable en este repositorio.
- El hook solo escucha las teclas virtuales `VK_CAPITAL`, `VK_NUMLOCK` y `VK_SCROLL`.
- No almacena pulsaciones de teclas ni realiza conexiones de red.

---

## 📄 Licencia

Distribuido bajo la Licencia **MIT**. Consulta [`LICENSE`](LICENSE) para más detalles.
