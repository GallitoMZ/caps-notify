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
    MF_CHECKED, MF_DISABLED, MF_GRAYED, MF_POPUP, MF_SEPARATOR, MF_STRING, MF_UNCHECKED,
    TPM_BOTTOMALIGN, TPM_RIGHTBUTTON, WM_APP,
};

pub const WM_APP_TRAY: u32 = WM_APP + 2;

pub const ID_TRAY_TITLE: usize = 2000;
pub const ID_TRAY_SETTINGS: usize = 2001;
pub const ID_TRAY_AUTOSTART: usize = 2002;
pub const ID_TRAY_OVERLAY: usize = 2003;
pub const ID_TRAY_TOASTS: usize = 2004;
pub const ID_TRAY_SOUND: usize = 2005;
pub const ID_TRAY_CONFIG: usize = 2006;
pub const ID_TRAY_ABOUT: usize = 2007;
pub const ID_TRAY_EXIT: usize = 2008;

pub const ID_POS_TOP_CENTER: usize = 2010;
pub const ID_POS_TOP_LEFT: usize = 2011;
pub const ID_POS_TOP_RIGHT: usize = 2012;
pub const ID_POS_CENTER_LEFT: usize = 2013;
pub const ID_POS_CENTER: usize = 2014;
pub const ID_POS_CENTER_RIGHT: usize = 2015;
pub const ID_POS_BOTTOM_LEFT: usize = 2016;
pub const ID_POS_BOTTOM_CENTER: usize = 2017;
pub const ID_POS_BOTTOM_RIGHT: usize = 2018;

pub const ID_SND_MODERN: usize = 2020;
pub const ID_SND_CLICK: usize = 2021;
pub const ID_SND_WIN: usize = 2022;

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
                // Header
                let _ = AppendMenuW(menu, MF_STRING | MF_DISABLED | MF_GRAYED, ID_TRAY_TITLE, w!("Caps Notify v0.1.0"));
                let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);

                // Open Settings Window
                let _ = AppendMenuW(menu, MF_STRING, ID_TRAY_SETTINGS, w!("Configuraci\u{00f3}n..."));
                let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);

                // HUD Position Submenu
                if let Ok(pos_menu) = CreatePopupMenu() {
                    let chk = |p: &str| if cfg.overlay_position == p { MF_CHECKED } else { MF_UNCHECKED };
                    let _ = AppendMenuW(pos_menu, MF_STRING | chk("TopCenter"), ID_POS_TOP_CENTER, w!("Arriba Centro (Por defecto)"));
                    let _ = AppendMenuW(pos_menu, MF_STRING | chk("TopLeft"), ID_POS_TOP_LEFT, w!("Arriba Izquierda"));
                    let _ = AppendMenuW(pos_menu, MF_STRING | chk("TopRight"), ID_POS_TOP_RIGHT, w!("Arriba Derecha"));
                    let _ = AppendMenuW(pos_menu, MF_SEPARATOR, 0, None);
                    let _ = AppendMenuW(pos_menu, MF_STRING | chk("CenterLeft"), ID_POS_CENTER_LEFT, w!("Centro Izquierda"));
                    let _ = AppendMenuW(pos_menu, MF_STRING | chk("Center"), ID_POS_CENTER, w!("Centro Total"));
                    let _ = AppendMenuW(pos_menu, MF_STRING | chk("CenterRight"), ID_POS_CENTER_RIGHT, w!("Centro Derecha"));
                    let _ = AppendMenuW(pos_menu, MF_SEPARATOR, 0, None);
                    let _ = AppendMenuW(pos_menu, MF_STRING | chk("BottomLeft"), ID_POS_BOTTOM_LEFT, w!("Abajo Izquierda"));
                    let _ = AppendMenuW(pos_menu, MF_STRING | chk("BottomCenter"), ID_POS_BOTTOM_CENTER, w!("Abajo Centro"));
                    let _ = AppendMenuW(pos_menu, MF_STRING | chk("BottomRight"), ID_POS_BOTTOM_RIGHT, w!("Abajo Derecha"));

                    let _ = AppendMenuW(menu, MF_POPUP, pos_menu.0 as usize, w!("Posici\u{00f3}n del HUD"));
                }

                // Sound Submenu
                if let Ok(snd_menu) = CreatePopupMenu() {
                    let chk_snd = |s: &str| if cfg.sound_theme == s { MF_CHECKED } else { MF_UNCHECKED };
                    let _ = AppendMenuW(snd_menu, MF_STRING | chk_snd("ModernChime"), ID_SND_MODERN, w!("Chime Moderno (Arm\u{00f3}nico)"));
                    let _ = AppendMenuW(snd_menu, MF_STRING | chk_snd("KeyClick"), ID_SND_CLICK, w!("Clic Mec\u{00e1}nico"));
                    let _ = AppendMenuW(snd_menu, MF_STRING | chk_snd("WindowsDefault"), ID_SND_WIN, w!("Sonido de Windows"));

                    let _ = AppendMenuW(menu, MF_POPUP, snd_menu.0 as usize, w!("Estilo de Sonido"));
                }

                let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);

                // Quick Toggles
                let overlay_flag = if cfg.overlay_enabled { MF_CHECKED } else { MF_UNCHECKED };
                let _ = AppendMenuW(menu, MF_STRING | overlay_flag, ID_TRAY_OVERLAY, w!("HUD Flotante"));

                let sound_flag = if cfg.sound_enabled { MF_CHECKED } else { MF_UNCHECKED };
                let _ = AppendMenuW(menu, MF_STRING | sound_flag, ID_TRAY_SOUND, w!("Sonido Chime"));

                let toast_flag = if cfg.toast_enabled { MF_CHECKED } else { MF_UNCHECKED };
                let _ = AppendMenuW(menu, MF_STRING | toast_flag, ID_TRAY_TOASTS, w!("Toasts de Windows"));

                let auto_flag = if is_autostart_active { MF_CHECKED } else { MF_UNCHECKED };
                let _ = AppendMenuW(menu, MF_STRING | auto_flag, ID_TRAY_AUTOSTART, w!("Autostart con Windows"));

                let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
                let _ = AppendMenuW(menu, MF_STRING, ID_TRAY_CONFIG, w!("Abrir config.toml"));
                let _ = AppendMenuW(menu, MF_STRING, ID_TRAY_ABOUT, w!("Acerca de Caps Notify"));
                let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
                let _ = AppendMenuW(menu, MF_STRING, ID_TRAY_EXIT, w!("Salir"));

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
