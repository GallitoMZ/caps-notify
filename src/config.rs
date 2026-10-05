use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub language: String,              // "en" (default) or "es"
    pub watch_caps: bool,
    pub watch_num: bool,
    pub watch_scroll: bool,
    pub toast_enabled: bool,
    pub sound_enabled: bool,
    pub sound_theme: String,           // "ModernChime", "KeyClick", "WindowsDefault"
    pub overlay_enabled: bool,         // default true
    pub overlay_position: String,      // "TopCenter", "TopRight", "TopLeft", etc.
    pub overlay_theme: String,         // "CapsNotifyModern" (default), "LenovoClassic"
    pub overlay_size: String,          // "Small", "Medium", "Large"
    pub overlay_duration_ms: u32,      // default 850
    pub autostart: bool,
    pub first_run: bool,               // true only on first run, then set to false
}

impl Default for Config {
    fn default() -> Self {
        Self {
            language: "en".to_string(), // English default as requested
            watch_caps: true,
            watch_num: true,
            watch_scroll: false,
            toast_enabled: false,
            sound_enabled: false,
            sound_theme: "ModernChime".to_string(),
            overlay_enabled: true,
            overlay_position: "TopCenter".to_string(),
            overlay_theme: "CapsNotifyModern".to_string(), // New unique design by default!
            overlay_size: "Medium".to_string(),
            overlay_duration_ms: 850,
            autostart: false,
            first_run: true,
        }
    }
}

impl Config {
    pub fn get_config_dir() -> PathBuf {
        if let Ok(appdata) = std::env::var("APPDATA") {
            PathBuf::from(appdata).join("caps-notify")
        } else {
            PathBuf::from(".").join("caps-notify")
        }
    }

    pub fn get_config_path() -> PathBuf {
        Self::get_config_dir().join("config.toml")
    }

    /// Loads config from %APPDATA%\caps-notify\config.toml, creating default if not found
    pub fn load() -> Self {
        let path = Self::get_config_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(cfg) = toml::from_str::<Config>(&content) {
                    return cfg;
                }
            }
        }

        let default_cfg = Config::default();
        let _ = default_cfg.save();
        default_cfg
    }

    /// Saves configuration to %APPDATA%\caps-notify\config.toml
    pub fn save(&self) -> Result<(), std::io::Error> {
        let dir = Self::get_config_dir();
        if !dir.exists() {
            fs::create_dir_all(&dir)?;
        }
        let content = toml::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        fs::write(Self::get_config_path(), content)?;
        Ok(())
    }
}
