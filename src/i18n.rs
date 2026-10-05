#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    English,
    Spanish,
}

#[allow(dead_code)]
impl Language {
    pub fn from_code(code: &str) -> Self {
        match code {
            "es" => Language::Spanish,
            _ => Language::English,
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Language::English => "en",
            Language::Spanish => "es",
        }
    }
}

#[allow(dead_code)]
pub struct I18n {
    pub lang: Language,
}

#[allow(dead_code)]
impl I18n {
    pub fn new(lang_code: &str) -> Self {
        Self {
            lang: Language::from_code(lang_code),
        }
    }

    pub fn status_on(&self) -> &'static str {
        match self.lang {
            Language::English => "ON",
            Language::Spanish => "ACTIVADO",
        }
    }

    pub fn status_off(&self) -> &'static str {
        match self.lang {
            Language::English => "OFF",
            Language::Spanish => "DESACTIVADO",
        }
    }

    pub fn caps_lock(&self) -> &'static str {
        match self.lang {
            Language::English => "Caps Lock",
            Language::Spanish => "Bloq Mayús",
        }
    }

    pub fn num_lock(&self) -> &'static str {
        match self.lang {
            Language::English => "Num Lock",
            Language::Spanish => "Bloq Num",
        }
    }

    pub fn scroll_lock(&self) -> &'static str {
        match self.lang {
            Language::English => "Scroll Lock",
            Language::Spanish => "Bloq Despl",
        }
    }

    pub fn settings_title(&self) -> &'static str {
        match self.lang {
            Language::English => "Caps Notify — Settings",
            Language::Spanish => "Caps Notify — Configuración",
        }
    }

    pub fn live_status_group(&self) -> &'static str {
        match self.lang {
            Language::English => " Real-Time Key Status ",
            Language::Spanish => " Estado en Tiempo Real ",
        }
    }

    pub fn hud_group(&self) -> &'static str {
        match self.lang {
            Language::English => " Floating HUD (On-Screen Display) ",
            Language::Spanish => " HUD Flotante (On-Screen Display) ",
        }
    }

    pub fn hud_enable(&self) -> &'static str {
        match self.lang {
            Language::English => "Enable On-Screen Display (HUD)",
            Language::Spanish => "Activar HUD flotante en pantalla",
        }
    }

    pub fn hud_theme_lbl(&self) -> &'static str {
        match self.lang {
            Language::English => "Visual Style:",
            Language::Spanish => "Tema visual:",
        }
    }

    pub fn hud_pos_lbl(&self) -> &'static str {
        match self.lang {
            Language::English => "Position:",
            Language::Spanish => "Posición:",
        }
    }

    pub fn hud_size_lbl(&self) -> &'static str {
        match self.lang {
            Language::English => "Size:",
            Language::Spanish => "Tamaño:",
        }
    }

    pub fn test_hud_btn(&self) -> &'static str {
        match self.lang {
            Language::English => "Test HUD",
            Language::Spanish => "Probar HUD",
        }
    }

    pub fn sound_group(&self) -> &'static str {
        match self.lang {
            Language::English => " Sound & Audio Feedback ",
            Language::Spanish => " Sonido y Notificaciones ",
        }
    }

    pub fn sound_enable(&self) -> &'static str {
        match self.lang {
            Language::English => "Play audio chime on key toggle",
            Language::Spanish => "Sonido al cambiar tecla",
        }
    }

    pub fn sound_style_lbl(&self) -> &'static str {
        match self.lang {
            Language::English => "Sound:",
            Language::Spanish => "Sonido:",
        }
    }

    pub fn test_sound_btn(&self) -> &'static str {
        match self.lang {
            Language::English => "Test Sound",
            Language::Spanish => "Probar sonido",
        }
    }

    pub fn toast_enable(&self) -> &'static str {
        match self.lang {
            Language::English => "Windows Toast notifications",
            Language::Spanish => "Notificaciones Toast de Windows",
        }
    }

    pub fn system_group(&self) -> &'static str {
        match self.lang {
            Language::English => " Language & System ",
            Language::Spanish => " Idioma y Sistema ",
        }
    }

    pub fn lang_lbl(&self) -> &'static str {
        match self.lang {
            Language::English => "Language:",
            Language::Spanish => "Idioma:",
        }
    }

    pub fn autostart_enable(&self) -> &'static str {
        match self.lang {
            Language::English => "Start with Windows (Autostart)",
            Language::Spanish => "Iniciar con Windows (Autostart)",
        }
    }

    pub fn save_btn(&self) -> &'static str {
        match self.lang {
            Language::English => "Save & Apply",
            Language::Spanish => "Guardar y Aplicar",
        }
    }

    pub fn close_btn(&self) -> &'static str {
        match self.lang {
            Language::English => "Close",
            Language::Spanish => "Cerrar",
        }
    }

    // Tray Menu
    pub fn tray_settings(&self) -> &'static str {
        match self.lang {
            Language::English => "Settings...",
            Language::Spanish => "Configuración...",
        }
    }

    pub fn tray_position(&self) -> &'static str {
        match self.lang {
            Language::English => "HUD Position",
            Language::Spanish => "Posición del HUD",
        }
    }

    pub fn tray_theme(&self) -> &'static str {
        match self.lang {
            Language::English => "HUD Theme",
            Language::Spanish => "Tema del HUD",
        }
    }

    pub fn tray_sound(&self) -> &'static str {
        match self.lang {
            Language::English => "Sound Style",
            Language::Spanish => "Estilo de Sonido",
        }
    }

    pub fn tray_hud_toggle(&self) -> &'static str {
        match self.lang {
            Language::English => "Floating HUD",
            Language::Spanish => "HUD Flotante",
        }
    }

    pub fn tray_sound_toggle(&self) -> &'static str {
        match self.lang {
            Language::English => "Sound Chime",
            Language::Spanish => "Sonido Chime",
        }
    }

    pub fn tray_autostart_toggle(&self) -> &'static str {
        match self.lang {
            Language::English => "Autostart on Boot",
            Language::Spanish => "Autostart con Windows",
        }
    }

    pub fn tray_open_config(&self) -> &'static str {
        match self.lang {
            Language::English => "Open config.toml",
            Language::Spanish => "Abrir config.toml",
        }
    }

    pub fn tray_about(&self) -> &'static str {
        match self.lang {
            Language::English => "About Caps Notify",
            Language::Spanish => "Acerca de Caps Notify",
        }
    }

    pub fn tray_exit(&self) -> &'static str {
        match self.lang {
            Language::English => "Exit",
            Language::Spanish => "Salir",
        }
    }

    pub fn about_body(&self) -> &'static str {
        match self.lang {
            Language::English => "Caps Notify v0.1.1\n\nUltra-lightweight native lock key indicator for Windows.\nZero polling, low RAM footprint.\n\nAuthor: GallitoMZ\nLicense: MIT",
            Language::Spanish => "Caps Notify v0.1.1\n\nIndicador nativo y ultra-ligero para teclas de bloqueo en Windows.\nCero polling, mínimo consumo de RAM.\n\nAutor: GallitoMZ\nLicencia: MIT",
        }
    }
}
