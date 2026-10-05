use std::collections::HashMap;
use crate::config::Config;
use crate::state::LockState;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY,
    NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, LoadIconW, LoadImageW,
    SetForegroundWindow, TrackPopupMenu, HICON, IDI_APPLICATION, IMAGE_ICON, LR_DEFAULTCOLOR,
    MF_CHECKED, MF_DISABLED, MF_GRAYED, MF_SEPARATOR, MF_STRING, MF_UNCHECKED, TPM_BOTTOMALIGN,
    TPM_RIGHTBUTTON, WM_APP,
};

pub const WM_APP_TRAY: u32 = WM_APP + 2;

pub const ID_TRAY_TITLE: usize = 2000;
pub const ID_TRAY_AUTOSTART: usize = 2001;
pub const ID_TRAY_OVERLAY: usize = 2002;
pub const ID_TRAY_TOASTS: usize = 2003;
pub const ID_TRAY_SOUND: usize = 2004;
pub const ID_TRAY_CONFIG: usize = 2005;
pub const ID_TRAY_ABOUT: usize = 2006;
pub const ID_TRAY_EXIT: usize = 2007;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IconKey {
    App,
    CapsOn,
    CapsOff,
    NumOn,
    NumOff,
    ScrollOn,
    ScrollOff,
}

pub struct Tray {
    hwnd: HWND,
    icons: HashMap<IconKey, HICON>,
}

impl Tray {
    pub fn new(hwnd: HWND, state: &LockState, cfg: &Config) -> Self {
        let icons = Self::load_all_icons();
        let tray = Self { hwnd, icons };
        tray.register(state, cfg);
        tray
    }

    fn load_all_icons() -> HashMap<IconKey, HICON> {
        let mut map = HashMap::new();
        let hinstance = unsafe { GetModuleHandleW(None).unwrap_or_default() };

        let load_res = |name: PCWSTR| -> HICON {
            unsafe {
                if let Ok(handle) = LoadImageW(
                    hinstance,
                    name,
                    IMAGE_ICON,
                    16,
                    16,
                    LR_DEFAULTCOLOR,
                ) {
                    HICON(handle.0)
                } else {
                    LoadIconW(None, IDI_APPLICATION).unwrap_or_default()
                }
            }
        };

        // App icon (MAKEINTRESOURCE(1))
        let app_icon = unsafe {
            if let Ok(h) = LoadImageW(
                hinstance,
                PCWSTR(1 as *const u16),
                IMAGE_ICON,
                16,
                16,
                LR_DEFAULTCOLOR,
            ) {
                HICON(h.0)
            } else {
                LoadIconW(None, IDI_APPLICATION).unwrap_or_default()
            }
        };

        map.insert(IconKey::App, app_icon);
        map.insert(IconKey::CapsOn, load_res(w!("CAPS_ON")));
        map.insert(IconKey::CapsOff, load_res(w!("CAPS_OFF")));
        map.insert(IconKey::NumOn, load_res(w!("NUM_ON")));
        map.insert(IconKey::NumOff, load_res(w!("NUM_OFF")));
        map.insert(IconKey::ScrollOn, load_res(w!("SCROLL_ON")));
        map.insert(IconKey::ScrollOff, load_res(w!("SCROLL_OFF")));

        map
    }

    fn select_icon(&self, state: &LockState, cfg: &Config) -> HICON {
        let key = if cfg.watch_caps {
            if state.is_caps_on() {
                IconKey::CapsOn
            } else {
                IconKey::CapsOff
            }
        } else if cfg.watch_num {
            if state.is_num_on() {
                IconKey::NumOn
            } else {
                IconKey::NumOff
            }
        } else if cfg.watch_scroll {
            if state.is_scroll_on() {
                IconKey::ScrollOn
            } else {
                IconKey::ScrollOff
            }
        } else {
            IconKey::App
        };

        self.icons.get(&key).copied().unwrap_or_default()
    }

    fn build_tooltip(&self, state: &LockState) -> [u16; 128] {
        let mut tip = [0u16; 128];
        let tip_str = format!(
            "Caps Notify\nCaps: {} | Num: {} | Scroll: {}",
            if state.is_caps_on() { "ON" } else { "OFF" },
            if state.is_num_on() { "ON" } else { "OFF" },
            if state.is_scroll_on() { "ON" } else { "OFF" }
        );

        for (i, c) in tip_str.encode_utf16().take(127).enumerate() {
            tip[i] = c;
        }
        tip
    }

    pub fn register(&self, state: &LockState, cfg: &Config) {
        let icon = self.select_icon(state, cfg);
        let tip = self.build_tooltip(state);

        let mut nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.hwnd,
            uID: 1,
            uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
            uCallbackMessage: WM_APP_TRAY,
            hIcon: icon,
            szTip: tip,
            ..Default::default()
        };

        unsafe {
            let _ = Shell_NotifyIconW(NIM_ADD, &mut nid);
        }
    }

    pub fn update(&self, state: &LockState, cfg: &Config) {
        let icon = self.select_icon(state, cfg);
        let tip = self.build_tooltip(state);

        let mut nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.hwnd,
            uID: 1,
            uFlags: NIF_ICON | NIF_TIP,
            hIcon: icon,
            szTip: tip,
            ..Default::default()
        };

        unsafe {
            let _ = Shell_NotifyIconW(NIM_MODIFY, &mut nid);
        }
    }

    pub fn remove(&self) {
        let mut nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.hwnd,
            uID: 1,
            ..Default::default()
        };

        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &mut nid);
        }
    }

    pub fn show_context_menu(&self, cfg: &Config, is_autostart_active: bool) {
        unsafe {
            let mut pt = POINT::default();
            let _ = GetCursorPos(&mut pt);

            if let Ok(menu) = CreatePopupMenu() {
                // Title item
                let _ = AppendMenuW(menu, MF_STRING | MF_DISABLED | MF_GRAYED, ID_TRAY_TITLE, w!("Caps Notify v0.1.0"));
                let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);

                // Autostart toggle
                let auto_flag = if is_autostart_active { MF_CHECKED } else { MF_UNCHECKED };
                let _ = AppendMenuW(menu, MF_STRING | auto_flag, ID_TRAY_AUTOSTART, w!("Autostart on Boot"));

                // Notification toggles
                let toast_flag = if cfg.toast_enabled { MF_CHECKED } else { MF_UNCHECKED };
                let _ = AppendMenuW(menu, MF_STRING | toast_flag, ID_TRAY_TOASTS, w!("Windows Toasts"));

                let sound_flag = if cfg.sound_enabled { MF_CHECKED } else { MF_UNCHECKED };
                let _ = AppendMenuW(menu, MF_STRING | sound_flag, ID_TRAY_SOUND, w!("Sound Chime"));

                let overlay_flag = if cfg.overlay_enabled { MF_CHECKED } else { MF_UNCHECKED };
                let _ = AppendMenuW(menu, MF_STRING | overlay_flag, ID_TRAY_OVERLAY, w!("HUD Floating Overlay"));

                let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
                let _ = AppendMenuW(menu, MF_STRING, ID_TRAY_CONFIG, w!("Open Config File"));
                let _ = AppendMenuW(menu, MF_STRING, ID_TRAY_ABOUT, w!("About Caps Notify"));
                let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
                let _ = AppendMenuW(menu, MF_STRING, ID_TRAY_EXIT, w!("Exit"));

                // Required before TrackPopupMenu so clicking outside dismisses the menu
                let _ = SetForegroundWindow(self.hwnd);
                let _ = TrackPopupMenu(
                    menu,
                    TPM_RIGHTBUTTON | TPM_BOTTOMALIGN,
                    pt.x,
                    pt.y,
                    0,
                    self.hwnd,
                    None,
                );
                let _ = DestroyMenu(menu);
            }
        }
    }
}

impl Drop for Tray {
    fn drop(&mut self) {
        self.remove();
    }
}
