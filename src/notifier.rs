use crate::config::Config;
use crate::overlay;
use crate::sound;
use windows::core::HSTRING;
use windows::Data::Xml::Dom::XmlDocument;
use windows::UI::Notifications::{ToastNotification, ToastNotificationManager};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockKey {
    Caps,
    Num,
    Scroll,
}

impl LockKey {
    pub fn name(&self) -> &'static str {
        match self {
            LockKey::Caps => "Caps Lock",
            LockKey::Num => "Num Lock",
            LockKey::Scroll => "Scroll Lock",
        }
    }
}

/// Dispatches notifications (HUD, Sound, Toasts) based on configuration
pub fn notify(key: LockKey, enabled: bool, cfg: &Config) {
    let state_str = if enabled { "ACTIVADO" } else { "DESACTIVADO" };
    let title = "Caps Notify";
    let message = format!("{} está ahora {}", key.name(), state_str);

    // 1. Floating HUD Overlay (Default enabled, inspired by Lenovo OSD)
    if cfg.overlay_enabled {
        overlay::show(key.name(), enabled, cfg);
    }

    // 2. High-fidelity harmonic audio chime / key click
    if cfg.sound_enabled {
        sound::play(&cfg.sound_theme, enabled);
    }

    // 3. Optional Toast Notification via WinRT
    if cfg.toast_enabled {
        let _ = show_toast(title, &message);
    }
}

fn show_toast(title: &str, body: &str) -> windows::core::Result<()> {
    let xml_content = format!(
        "<toast><visual><binding template=\"ToastGeneric\"><text>{}</text><text>{}</text></binding></visual></toast>",
        escape_xml(title),
        escape_xml(body)
    );

    let doc = XmlDocument::new()?;
    doc.LoadXml(&HSTRING::from(xml_content))?;

    let toast = ToastNotification::CreateToastNotification(&doc)?;
    let notifier = ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from("CapsNotify"))?;
    notifier.Show(&toast)?;
    Ok(())
}

fn escape_xml(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
