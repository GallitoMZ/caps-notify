use std::collections::HashMap;
use crate::config::Config;
use crate::i18n::{I18n, Language};
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

pub const ID_THM_MODERN: usize = 2030;
pub const ID_THM_LENOVO: usize = 2031;

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

    fn build_tooltip(&self, state: &LockState, cfg: &Config) -> [u16; 128] {
        let mut tip = [0u16; 128];
        let i18n = I18n::new(&cfg.language);
        let (on_t, off_t) = (i18n.status_on(), i18n.status_off());

        let tip_str = format!(
            "Caps Notify\n{}: {} | {}: {} | {}: {}",
            i18n.caps_lock(),
            if state.is_caps_on() { on_t } else { off_t },
            i18n.num_lock(),
            if state.is_num_on() { on_t } else { off_t },
            i18n.scroll_lock(),
            if state.is_scroll_on() { on_t } else { off_t }
        );

        for (i, c) in tip_str.encode_utf16().take(127).enumerate() {
            tip[i] = c;
        }
        tip
    }

    pub fn register(&self, state: &LockState, cfg: &Config) {
        let icon = self.select_icon(state, cfg);
        let tip = self.build_tooltip(state, cfg);

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
        let tip = self.build_tooltip(state, cfg);

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
            let i18n = I18n::new(&cfg.language);

            let to_w = |s: &str| -> Vec<u16> {
                s.encode_utf16().chain(std::iter::once(0)).collect()
            };

            if let Ok(menu) = CreatePopupMenu() {
                let _ = AppendMenuW(menu, MF_STRING | MF_DISABLED | MF_GRAYED, ID_TRAY_TITLE, w!("Caps Notify v0.1.1"));
                let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);

                let _ = AppendMenuW(menu, MF_STRING, ID_TRAY_SETTINGS, PCWSTR(to_w(i18n.tray_settings()).as_ptr()));
                let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);

                // Theme Submenu
                if let Ok(thm_menu) = CreatePopupMenu() {
                    let chk_thm = |t: &str| if cfg.overlay_theme == t { MF_CHECKED } else { MF_UNCHECKED };
                    let lbl_m = if i18n.lang == Language::Spanish { "Caps Notify Moderno (Default)" } else { "Caps Notify Modern (Default)" };
                    let lbl_l = if i18n.lang == Language::Spanish { "Lenovo Cl\u{00e1}sico (OSD)" } else { "Lenovo Classic (OSD)" };
                    let _ = AppendMenuW(thm_menu, MF_STRING | chk_thm("CapsNotifyModern"), ID_THM_MODERN, PCWSTR(to_w(lbl_m).as_ptr()));
                    let _ = AppendMenuW(thm_menu, MF_STRING | chk_thm("LenovoClassic"), ID_THM_LENOVO, PCWSTR(to_w(lbl_l).as_ptr()));
                    let _ = AppendMenuW(menu, MF_POPUP, thm_menu.0 as usize, PCWSTR(to_w(i18n.tray_theme()).as_ptr()));
                }

                // Position Submenu
                if let Ok(pos_menu) = CreatePopupMenu() {
                    let chk = |p: &str| if cfg.overlay_position == p { MF_CHECKED } else { MF_UNCHECKED };
                    let (tc, tl, tr, cl, c, cr, bl, bc, br) = if i18n.lang == Language::Spanish {
                        ("Arriba Centro (Default)", "Arriba Izquierda", "Arriba Derecha", "Centro Izquierda", "Centro Total", "Centro Derecha", "Abajo Izquierda", "Abajo Centro", "Abajo Derecha")
                    } else {
                        ("Top Center (Default)", "Top Left", "Top Right", "Center Left", "Center", "Center Right", "Bottom Left", "Bottom Center", "Bottom Right")
                    };

                    let _ = AppendMenuW(pos_menu, MF_STRING | chk("TopCenter"), ID_POS_TOP_CENTER, PCWSTR(to_w(tc).as_ptr()));
                    let _ = AppendMenuW(pos_menu, MF_STRING | chk("TopLeft"), ID_POS_TOP_LEFT, PCWSTR(to_w(tl).as_ptr()));
                    let _ = AppendMenuW(pos_menu, MF_STRING | chk("TopRight"), ID_POS_TOP_RIGHT, PCWSTR(to_w(tr).as_ptr()));
                    let _ = AppendMenuW(pos_menu, MF_SEPARATOR, 0, None);
                    let _ = AppendMenuW(pos_menu, MF_STRING | chk("CenterLeft"), ID_POS_CENTER_LEFT, PCWSTR(to_w(cl).as_ptr()));
                    let _ = AppendMenuW(pos_menu, MF_STRING | chk("Center"), ID_POS_CENTER, PCWSTR(to_w(c).as_ptr()));
                    let _ = AppendMenuW(pos_menu, MF_STRING | chk("CenterRight"), ID_POS_CENTER_RIGHT, PCWSTR(to_w(cr).as_ptr()));
                    let _ = AppendMenuW(pos_menu, MF_SEPARATOR, 0, None);
                    let _ = AppendMenuW(pos_menu, MF_STRING | chk("BottomLeft"), ID_POS_BOTTOM_LEFT, PCWSTR(to_w(bl).as_ptr()));
                    let _ = AppendMenuW(pos_menu, MF_STRING | chk("BottomCenter"), ID_POS_BOTTOM_CENTER, PCWSTR(to_w(bc).as_ptr()));
                    let _ = AppendMenuW(pos_menu, MF_STRING | chk("BottomRight"), ID_POS_BOTTOM_RIGHT, PCWSTR(to_w(br).as_ptr()));

                    let _ = AppendMenuW(menu, MF_POPUP, pos_menu.0 as usize, PCWSTR(to_w(i18n.tray_position()).as_ptr()));
                }

                // Sound Submenu
                if let Ok(snd_menu) = CreatePopupMenu() {
                    let chk_snd = |s: &str| if cfg.sound_theme == s { MF_CHECKED } else { MF_UNCHECKED };
                    let (s1, s2, s3) = if i18n.lang == Language::Spanish {
                        ("Chime Moderno (Arm\u{00f3}nico)", "Clic Mec\u{00e1}nico", "Sonido de Windows")
                    } else {
                        ("Modern Chime (Harmonic)", "Mechanical Click", "Windows Sound")
                    };
                    let _ = AppendMenuW(snd_menu, MF_STRING | chk_snd("ModernChime"), ID_SND_MODERN, PCWSTR(to_w(s1).as_ptr()));
                    let _ = AppendMenuW(snd_menu, MF_STRING | chk_snd("KeyClick"), ID_SND_CLICK, PCWSTR(to_w(s2).as_ptr()));
                    let _ = AppendMenuW(snd_menu, MF_STRING | chk_snd("WindowsDefault"), ID_SND_WIN, PCWSTR(to_w(s3).as_ptr()));

                    let _ = AppendMenuW(menu, MF_POPUP, snd_menu.0 as usize, PCWSTR(to_w(i18n.tray_sound()).as_ptr()));
                }

                let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);

                let overlay_flag = if cfg.overlay_enabled { MF_CHECKED } else { MF_UNCHECKED };
                let _ = AppendMenuW(menu, MF_STRING | overlay_flag, ID_TRAY_OVERLAY, PCWSTR(to_w(i18n.tray_hud_toggle()).as_ptr()));

                let sound_flag = if cfg.sound_enabled { MF_CHECKED } else { MF_UNCHECKED };
                let _ = AppendMenuW(menu, MF_STRING | sound_flag, ID_TRAY_SOUND, PCWSTR(to_w(i18n.tray_sound_toggle()).as_ptr()));

                let auto_flag = if is_autostart_active { MF_CHECKED } else { MF_UNCHECKED };
                let _ = AppendMenuW(menu, MF_STRING | auto_flag, ID_TRAY_AUTOSTART, PCWSTR(to_w(i18n.tray_autostart_toggle()).as_ptr()));

                let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
                let _ = AppendMenuW(menu, MF_STRING, ID_TRAY_CONFIG, PCWSTR(to_w(i18n.tray_open_config()).as_ptr()));
                let _ = AppendMenuW(menu, MF_STRING, ID_TRAY_ABOUT, PCWSTR(to_w(i18n.tray_about()).as_ptr()));
                let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
                let _ = AppendMenuW(menu, MF_STRING, ID_TRAY_EXIT, PCWSTR(to_w(i18n.tray_exit()).as_ptr()));

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
