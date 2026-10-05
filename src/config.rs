use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub watch_caps: bool,
    pub watch_num: bool,
    pub watch_scroll: bool,
    pub toast_enabled: bool,
    pub sound_enabled: bool,
    pub overlay_enabled: bool,
    pub overlay_position: String,
    pub overlay_duration_ms: u32,
    pub autostart: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            watch_caps: true,
            watch_num: true,
            watch_scroll: false,
            toast_enabled: true,
            sound_enabled: false,
            overlay_enabled: false,
            overlay_position: "TopCenter".to_string(),
            overlay_duration_ms: 800,
            autostart: false,
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
