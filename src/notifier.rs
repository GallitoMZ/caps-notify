use crate::config::Config;
use crate::overlay;
use windows::core::{w, HSTRING};
use windows::Data::Xml::Dom::XmlDocument;
use windows::UI::Notifications::{ToastNotification, ToastNotificationManager};
use windows::Win32::Media::Audio::{PlaySoundW, SND_ALIAS, SND_ASYNC, SND_NODEFAULT};

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

/// Dispatches notifications (Toasts, Sound, Overlay) based on configuration
pub fn notify(key: LockKey, enabled: bool, cfg: &Config) {
    let state_str = if enabled { "ON" } else { "OFF" };
    let title = "Caps Notify";
    let message = format!("{} is now {}", key.name(), state_str);

    // 1. Toast Notification via WinRT
    if cfg.toast_enabled {
        let _ = show_toast(title, &message);
    }

    // 2. Audio playback
    if cfg.sound_enabled {
        play_sound();
    }

    // 3. Floating HUD overlay
    if cfg.overlay_enabled {
        overlay::show(key.name(), enabled, &cfg.overlay_position, cfg.overlay_duration_ms);
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
    // Use App User Model ID
    let notifier = ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from("CapsNotify"))?;
    notifier.Show(&toast)?;
    Ok(())
}

fn play_sound() {
    unsafe {
        // Native Windows notification chime
        let _ = PlaySoundW(
            w!("SystemNotification"),
            None,
            SND_ALIAS | SND_ASYNC | SND_NODEFAULT,
        );
    }
}

fn escape_xml(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
