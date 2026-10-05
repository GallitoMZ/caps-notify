use crate::autostart;
use crate::config::Config;
use crate::i18n::{I18n, Language};
use crate::overlay;
use crate::sound;
use crate::state::STATE;

use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::Mutex;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{CreateFontW, DeleteObject, FW_BOLD, FW_NORMAL, HFONT};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
    GetSystemMetrics, IsDialogMessageW, LoadImageW, PostQuitMessage, RegisterClassW,
    SendMessageW, SetForegroundWindow, SetWindowTextW, ShowWindow, TranslateMessage, CS_HREDRAW,
    CS_VREDRAW, HICON, HMENU, ICON_BIG, ICON_SMALL, IMAGE_ICON, LR_DEFAULTCOLOR, MSG,
    SM_CXSCREEN, SM_CYSCREEN, SW_SHOW, WINDOW_STYLE, WM_COMMAND, WM_CREATE, WM_DESTROY,
    WM_SETFONT, WM_SETICON, WNDCLASSW, WS_CAPTION, WS_CHILD, WS_MINIMIZEBOX, WS_OVERLAPPED,
    WS_SYSMENU, WS_VISIBLE,
};

const SETTINGS_CLASS_NAME: PCWSTR = w!("CapsNotifySettingsWindow");

// Win32 Control Styles & Messages
const BS_AUTOCHECKBOX: u32 = 0x00000003;
const BS_GROUPBOX: u32 = 0x00000007;
const CBS_DROPDOWNLIST: u32 = 0x0003;
const BM_GETCHECK: u32 = 0x00F0;
const BM_SETCHECK: u32 = 0x00F1;
const BST_CHECKED: usize = 1;
const BST_UNCHECKED: usize = 0;
const CB_ADDSTRING: u32 = 0x0143;
const CB_SETCURSEL: u32 = 0x014E;
const CB_GETCURSEL: u32 = 0x0147;

// Control IDs
const ID_CHK_OVERLAY: usize = 101;
const ID_CMB_THEME: usize = 102;
const ID_CMB_POSITION: usize = 103;
const ID_CMB_SIZE: usize = 104;
const ID_BTN_TEST_HUD: usize = 105;

const ID_CHK_SOUND: usize = 201;
const ID_CMB_SOUND: usize = 202;
const ID_BTN_TEST_SOUND: usize = 203;
const ID_CHK_TOAST: usize = 204;

const ID_CMB_LANG: usize = 301;
const ID_CHK_AUTOSTART: usize = 302;

const ID_BTN_SAVE: usize = 401;
const ID_BTN_CLOSE: usize = 402;

static SETTINGS_HWND: AtomicIsize = AtomicIsize::new(0);
static CURRENT_CONFIG: Mutex<Option<Config>> = Mutex::new(None);
static ON_SAVE_CALLBACK: Mutex<Option<Box<dyn Fn(Config) + Send + Sync>>> = Mutex::new(None);

// Control HWNDs stored as raw isize to implement Send + Sync
struct SettingsControls {
    chk_overlay: isize,
    cmb_theme: isize,
    cmb_position: isize,
    cmb_size: isize,
    chk_sound: isize,
    cmb_sound: isize,
    chk_toast: isize,
    cmb_lang: isize,
    chk_autostart: isize,
    lbl_caps_val: isize,
    lbl_num_val: isize,
    lbl_scroll_val: isize,
    font_handle: isize,
    font_bold_handle: isize,
}

static CONTROLS: Mutex<Option<SettingsControls>> = Mutex::new(None);

pub fn open_settings<F: Fn(Config) + Send + Sync + 'static>(cfg: Config, on_save: F) {
    let existing = SETTINGS_HWND.load(Ordering::SeqCst);
    if existing != 0 {
        let hwnd = HWND(existing as *mut _);
        unsafe {
            let _ = ShowWindow(hwnd, SW_SHOW);
            let _ = SetForegroundWindow(hwnd);
        }
        return;
    }

    {
        let mut g = CURRENT_CONFIG.lock().unwrap();
        *g = Some(cfg);
        let mut cb = ON_SAVE_CALLBACK.lock().unwrap();
        *cb = Some(Box::new(on_save));
    }

    std::thread::spawn(|| {
        run_settings_thread();
    });
}

fn run_settings_thread() {
    unsafe {
        let instance = GetModuleHandleW(None).unwrap_or_default();
        let app_icon = LoadImageW(
            instance,
            PCWSTR(1 as *const u16),
            IMAGE_ICON,
            32,
            32,
            LR_DEFAULTCOLOR,
        ).map(|h| HICON(h.0)).unwrap_or_default();

        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(settings_wndproc),
            hInstance: instance.into(),
            hIcon: app_icon,
            lpszClassName: SETTINGS_CLASS_NAME,
            hbrBackground: windows::Win32::Graphics::Gdi::HBRUSH((windows::Win32::Graphics::Gdi::COLOR_BTNFACE.0 + 1) as *mut _),
            ..Default::default()
        };
        let _ = RegisterClassW(&wc);

        let win_w = 480;
        let win_h = 580;
        let screen_w = GetSystemMetrics(SM_CXSCREEN);
        let screen_h = GetSystemMetrics(SM_CYSCREEN);
        let x = (screen_w - win_w) / 2;
        let y = (screen_h - win_h) / 2;

        let cfg = CURRENT_CONFIG.lock().unwrap().clone().unwrap_or_default();
        let i18n = I18n::new(&cfg.language);
        let wide_title: Vec<u16> = i18n.settings_title().encode_utf16().chain(std::iter::once(0)).collect();

        let hwnd = CreateWindowExW(
            Default::default(),
            SETTINGS_CLASS_NAME,
            PCWSTR(wide_title.as_ptr()),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
            x,
            y,
            win_w,
            win_h,
            None,
            None,
            instance,
            None,
        ).unwrap_or_default();

        if hwnd.0.is_null() {
            return;
        }

        // Set application icon in title bar and taskbar
        let app_icon_sm = LoadImageW(
            instance,
            PCWSTR(1 as *const u16),
            IMAGE_ICON,
            16,
            16,
            LR_DEFAULTCOLOR,
        ).map(|h| HICON(h.0)).unwrap_or_default();

        SendMessageW(hwnd, WM_SETICON, WPARAM(ICON_BIG as usize), LPARAM(app_icon.0 as isize));
        SendMessageW(hwnd, WM_SETICON, WPARAM(ICON_SMALL as usize), LPARAM(app_icon_sm.0 as isize));

        SETTINGS_HWND.store(hwnd.0 as isize, Ordering::SeqCst);
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetForegroundWindow(hwnd);

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).into() {
            if !IsDialogMessageW(hwnd, &msg).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        SETTINGS_HWND.store(0, Ordering::SeqCst);
    }
}

pub fn update_live_status() {
    let existing = SETTINGS_HWND.load(Ordering::SeqCst);
    if existing == 0 {
        return;
    }

    if let Ok(guard) = CONTROLS.lock() {
        if let Some(c) = guard.as_ref() {
            let cfg = CURRENT_CONFIG.lock().unwrap().clone().unwrap_or_default();
            let i18n = I18n::new(&cfg.language);

            let (on_txt, off_txt) = match i18n.lang {
                Language::Spanish => ("● ACTIVADO", "○ DESACTIVADO"),
                Language::English => ("● ON", "○ OFF"),
            };

            let caps_txt: Vec<u16> = (if STATE.is_caps_on() { on_txt } else { off_txt }).encode_utf16().chain(std::iter::once(0)).collect();
            let num_txt: Vec<u16> = (if STATE.is_num_on() { on_txt } else { off_txt }).encode_utf16().chain(std::iter::once(0)).collect();
            let scroll_txt: Vec<u16> = (if STATE.is_scroll_on() { on_txt } else { off_txt }).encode_utf16().chain(std::iter::once(0)).collect();

            unsafe {
                let _ = SetWindowTextW(HWND(c.lbl_caps_val as *mut _), PCWSTR(caps_txt.as_ptr()));
                let _ = SetWindowTextW(HWND(c.lbl_num_val as *mut _), PCWSTR(num_txt.as_ptr()));
                let _ = SetWindowTextW(HWND(c.lbl_scroll_val as *mut _), PCWSTR(scroll_txt.as_ptr()));
            }
        }
    }
}

unsafe extern "system" fn settings_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            let instance = GetModuleHandleW(None).unwrap_or_default();
            let cfg = {
                CURRENT_CONFIG.lock().unwrap().clone().unwrap_or_default()
            };
            let i18n = I18n::new(&cfg.language);

            // Create clean modern Segoe UI fonts
            let hfont = CreateFontW(
                15, 0, 0, 0, FW_NORMAL.0 as i32, 0, 0, 0, 0, 0, 0, 0, 0, w!("Segoe UI"),
            );
            let hfont_bold = CreateFontW(
                15, 0, 0, 0, FW_BOLD.0 as i32, 0, 0, 0, 0, 0, 0, 0, 0, w!("Segoe UI"),
            );

            let set_f = |ctrl: HWND| {
                SendMessageW(ctrl, WM_SETFONT, WPARAM(hfont.0 as usize), LPARAM(1));
            };
            let set_fb = |ctrl: HWND| {
                SendMessageW(ctrl, WM_SETFONT, WPARAM(hfont_bold.0 as usize), LPARAM(1));
            };

            let to_w = |s: &str| -> Vec<u16> {
                s.encode_utf16().chain(std::iter::once(0)).collect()
            };

            // -------------------------------------------------------------
            // Group 1: Real-Time Status Cards (Spacious, clean 3-card layout)
            // -------------------------------------------------------------
            let grp1 = CreateWindowExW(
                Default::default(), w!("BUTTON"), PCWSTR(to_w(i18n.live_status_group()).as_ptr()),
                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_GROUPBOX),
                16, 10, 432, 85, hwnd, None, instance, None,
            ).unwrap_or_default();
            set_fb(grp1);

            // Column 1: Caps Lock
            let lbl_c_title = CreateWindowExW(Default::default(), w!("STATIC"), PCWSTR(to_w(i18n.caps_lock()).as_ptr()), WS_CHILD | WS_VISIBLE, 30, 32, 120, 20, hwnd, None, instance, None).unwrap_or_default();
            set_fb(lbl_c_title);
            let (on_t, off_t) = match i18n.lang { Language::Spanish => ("● ACTIVADO", "○ DESACTIVADO"), Language::English => ("● ON", "○ OFF") };
            let lbl_c_val = CreateWindowExW(Default::default(), w!("STATIC"), PCWSTR(to_w(if STATE.is_caps_on() { on_t } else { off_t }).as_ptr()), WS_CHILD | WS_VISIBLE, 30, 54, 120, 22, hwnd, None, instance, None).unwrap_or_default();
            set_f(lbl_c_val);

            // Column 2: Num Lock
            let lbl_n_title = CreateWindowExW(Default::default(), w!("STATIC"), PCWSTR(to_w(i18n.num_lock()).as_ptr()), WS_CHILD | WS_VISIBLE, 170, 32, 120, 20, hwnd, None, instance, None).unwrap_or_default();
            set_fb(lbl_n_title);
            let lbl_n_val = CreateWindowExW(Default::default(), w!("STATIC"), PCWSTR(to_w(if STATE.is_num_on() { on_t } else { off_t }).as_ptr()), WS_CHILD | WS_VISIBLE, 170, 54, 120, 22, hwnd, None, instance, None).unwrap_or_default();
            set_f(lbl_n_val);

            // Column 3: Scroll Lock
            let lbl_s_title = CreateWindowExW(Default::default(), w!("STATIC"), PCWSTR(to_w(i18n.scroll_lock()).as_ptr()), WS_CHILD | WS_VISIBLE, 310, 32, 120, 20, hwnd, None, instance, None).unwrap_or_default();
            set_fb(lbl_s_title);
            let lbl_s_val = CreateWindowExW(Default::default(), w!("STATIC"), PCWSTR(to_w(if STATE.is_scroll_on() { on_t } else { off_t }).as_ptr()), WS_CHILD | WS_VISIBLE, 310, 54, 120, 22, hwnd, None, instance, None).unwrap_or_default();
            set_f(lbl_s_val);

            // -------------------------------------------------------------
            // Group 2: Floating HUD (On-Screen Display)
            // -------------------------------------------------------------
            let grp2 = CreateWindowExW(
                Default::default(), w!("BUTTON"), PCWSTR(to_w(i18n.hud_group()).as_ptr()),
                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_GROUPBOX),
                16, 105, 432, 185, hwnd, None, instance, None,
            ).unwrap_or_default();
            set_fb(grp2);

            let chk_overlay = CreateWindowExW(
                Default::default(), w!("BUTTON"), PCWSTR(to_w(i18n.hud_enable()).as_ptr()),
                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_AUTOCHECKBOX),
                30, 130, 270, 22, hwnd, HMENU(ID_CHK_OVERLAY as *mut _), instance, None,
            ).unwrap_or_default();
            set_f(chk_overlay);
            SendMessageW(chk_overlay, BM_SETCHECK, WPARAM(if cfg.overlay_enabled { BST_CHECKED } else { BST_UNCHECKED }), LPARAM(0));

            // Theme Combo
            let lbl_thm = CreateWindowExW(Default::default(), w!("STATIC"), PCWSTR(to_w(i18n.hud_theme_lbl()).as_ptr()), WS_CHILD | WS_VISIBLE, 30, 164, 85, 20, hwnd, None, instance, None).unwrap_or_default();
            set_f(lbl_thm);
            let cmb_thm = CreateWindowExW(
                Default::default(), w!("COMBOBOX"), None,
                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(CBS_DROPDOWNLIST),
                120, 160, 185, 140, hwnd, HMENU(ID_CMB_THEME as *mut _), instance, None,
            ).unwrap_or_default();
            set_f(cmb_thm);

            let themes = [
                ("CapsNotifyModern", if i18n.lang == Language::Spanish { "Caps Notify Moderno (Default)" } else { "Caps Notify Modern (Default)" }),
                ("LenovoClassic", if i18n.lang == Language::Spanish { "Lenovo Cl\u{00e1}sico (OSD Original)" } else { "Lenovo Classic (Exact OSD)" }),
            ];
            let mut sel_thm = 0;
            for (i, (code, lbl)) in themes.iter().enumerate() {
                SendMessageW(cmb_thm, CB_ADDSTRING, WPARAM(0), LPARAM(to_w(lbl).as_ptr() as isize));
                if *code == cfg.overlay_theme {
                    sel_thm = i;
                }
            }
            SendMessageW(cmb_thm, CB_SETCURSEL, WPARAM(sel_thm), LPARAM(0));

            // Position Combo
            let lbl_pos = CreateWindowExW(Default::default(), w!("STATIC"), PCWSTR(to_w(i18n.hud_pos_lbl()).as_ptr()), WS_CHILD | WS_VISIBLE, 30, 198, 85, 20, hwnd, None, instance, None).unwrap_or_default();
            set_f(lbl_pos);
            let cmb_pos = CreateWindowExW(
                Default::default(), w!("COMBOBOX"), None,
                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(CBS_DROPDOWNLIST),
                120, 194, 185, 200, hwnd, HMENU(ID_CMB_POSITION as *mut _), instance, None,
            ).unwrap_or_default();
            set_f(cmb_pos);

            let positions = [
                ("TopCenter", if i18n.lang == Language::Spanish { "Arriba Centro (Default)" } else { "Top Center (Default)" }),
                ("TopLeft", if i18n.lang == Language::Spanish { "Arriba Izquierda" } else { "Top Left" }),
                ("TopRight", if i18n.lang == Language::Spanish { "Arriba Derecha" } else { "Top Right" }),
                ("CenterLeft", if i18n.lang == Language::Spanish { "Centro Izquierda" } else { "Center Left" }),
                ("Center", if i18n.lang == Language::Spanish { "Centro Total" } else { "Center" }),
                ("CenterRight", if i18n.lang == Language::Spanish { "Centro Derecha" } else { "Center Right" }),
                ("BottomLeft", if i18n.lang == Language::Spanish { "Abajo Izquierda" } else { "Bottom Left" }),
                ("BottomCenter", if i18n.lang == Language::Spanish { "Abajo Centro" } else { "Bottom Center" }),
                ("BottomRight", if i18n.lang == Language::Spanish { "Abajo Derecha" } else { "Bottom Right" }),
            ];
            let mut sel_pos = 0;
            for (i, (code, lbl)) in positions.iter().enumerate() {
                SendMessageW(cmb_pos, CB_ADDSTRING, WPARAM(0), LPARAM(to_w(lbl).as_ptr() as isize));
                if *code == cfg.overlay_position {
                    sel_pos = i;
                }
            }
            SendMessageW(cmb_pos, CB_SETCURSEL, WPARAM(sel_pos), LPARAM(0));

            // Size Combo
            let lbl_sz = CreateWindowExW(Default::default(), w!("STATIC"), PCWSTR(to_w(i18n.hud_size_lbl()).as_ptr()), WS_CHILD | WS_VISIBLE, 30, 232, 85, 20, hwnd, None, instance, None).unwrap_or_default();
            set_f(lbl_sz);
            let cmb_sz = CreateWindowExW(
                Default::default(), w!("COMBOBOX"), None,
                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(CBS_DROPDOWNLIST),
                120, 228, 185, 120, hwnd, HMENU(ID_CMB_SIZE as *mut _), instance, None,
            ).unwrap_or_default();
            set_f(cmb_sz);

            let sizes = [
                ("Small", if i18n.lang == Language::Spanish { "Compacto (108 px)" } else { "Compact (108 px)" }),
                ("Medium", if i18n.lang == Language::Spanish { "Normal (130 px)" } else { "Normal (130 px)" }),
                ("Large", if i18n.lang == Language::Spanish { "Grande (156 px)" } else { "Large (156 px)" }),
            ];
            let mut sel_sz = 1;
            for (i, (code, lbl)) in sizes.iter().enumerate() {
                SendMessageW(cmb_sz, CB_ADDSTRING, WPARAM(0), LPARAM(to_w(lbl).as_ptr() as isize));
                if *code == cfg.overlay_size {
                    sel_sz = i;
                }
            }
            SendMessageW(cmb_sz, CB_SETCURSEL, WPARAM(sel_sz), LPARAM(0));

            // Test HUD Button
            let btn_test_hud = CreateWindowExW(
                Default::default(), w!("BUTTON"), PCWSTR(to_w(i18n.test_hud_btn()).as_ptr()),
                WS_CHILD | WS_VISIBLE,
                320, 160, 112, 32, hwnd, HMENU(ID_BTN_TEST_HUD as *mut _), instance, None,
            ).unwrap_or_default();
            set_f(btn_test_hud);

            // -------------------------------------------------------------
            // Group 3: Audio & Notifications
            // -------------------------------------------------------------
            let grp3 = CreateWindowExW(
                Default::default(), w!("BUTTON"), PCWSTR(to_w(i18n.sound_group()).as_ptr()),
                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_GROUPBOX),
                16, 300, 432, 118, hwnd, None, instance, None,
            ).unwrap_or_default();
            set_fb(grp3);

            let chk_snd = CreateWindowExW(
                Default::default(), w!("BUTTON"), PCWSTR(to_w(i18n.sound_enable()).as_ptr()),
                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_AUTOCHECKBOX),
                30, 324, 270, 22, hwnd, HMENU(ID_CHK_SOUND as *mut _), instance, None,
            ).unwrap_or_default();
            set_f(chk_snd);
            SendMessageW(chk_snd, BM_SETCHECK, WPARAM(if cfg.sound_enabled { BST_CHECKED } else { BST_UNCHECKED }), LPARAM(0));

            let cmb_snd = CreateWindowExW(
                Default::default(), w!("COMBOBOX"), None,
                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(CBS_DROPDOWNLIST),
                30, 354, 200, 120, hwnd, HMENU(ID_CMB_SOUND as *mut _), instance, None,
            ).unwrap_or_default();
            set_f(cmb_snd);

            let sound_themes = [
                ("ModernChime", if i18n.lang == Language::Spanish { "Chime Moderno (Arm\u{00f3}nico)" } else { "Modern Chime (Harmonic)" }),
                ("KeyClick", if i18n.lang == Language::Spanish { "Clic Mec\u{00e1}nico" } else { "Mechanical Click" }),
                ("WindowsDefault", if i18n.lang == Language::Spanish { "Sonido de Windows" } else { "Windows Default Sound" }),
            ];
            let mut sel_snd = 0;
            for (i, (code, lbl)) in sound_themes.iter().enumerate() {
                SendMessageW(cmb_snd, CB_ADDSTRING, WPARAM(0), LPARAM(to_w(lbl).as_ptr() as isize));
                if *code == cfg.sound_theme {
                    sel_snd = i;
                }
            }
            SendMessageW(cmb_snd, CB_SETCURSEL, WPARAM(sel_snd), LPARAM(0));

            let btn_test_snd = CreateWindowExW(
                Default::default(), w!("BUTTON"), PCWSTR(to_w(i18n.test_sound_btn()).as_ptr()),
                WS_CHILD | WS_VISIBLE,
                245, 353, 105, 26, hwnd, HMENU(ID_BTN_TEST_SOUND as *mut _), instance, None,
            ).unwrap_or_default();
            set_f(btn_test_snd);

            let chk_tst = CreateWindowExW(
                Default::default(), w!("BUTTON"), PCWSTR(to_w(i18n.toast_enable()).as_ptr()),
                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_AUTOCHECKBOX),
                30, 386, 270, 22, hwnd, HMENU(ID_CHK_TOAST as *mut _), instance, None,
            ).unwrap_or_default();
            set_f(chk_tst);
            SendMessageW(chk_tst, BM_SETCHECK, WPARAM(if cfg.toast_enabled { BST_CHECKED } else { BST_UNCHECKED }), LPARAM(0));

            // -------------------------------------------------------------
            // Group 4: Language & System Options
            // -------------------------------------------------------------
            let grp4 = CreateWindowExW(
                Default::default(), w!("BUTTON"), PCWSTR(to_w(i18n.system_group()).as_ptr()),
                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_GROUPBOX),
                16, 428, 432, 65, hwnd, None, instance, None,
            ).unwrap_or_default();
            set_fb(grp4);

            // Language Selector
            let lbl_lang = CreateWindowExW(Default::default(), w!("STATIC"), PCWSTR(to_w(i18n.lang_lbl()).as_ptr()), WS_CHILD | WS_VISIBLE, 30, 456, 75, 20, hwnd, None, instance, None).unwrap_or_default();
            set_f(lbl_lang);
            let cmb_lng = CreateWindowExW(
                Default::default(), w!("COMBOBOX"), None,
                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(CBS_DROPDOWNLIST),
                110, 452, 105, 100, hwnd, HMENU(ID_CMB_LANG as *mut _), instance, None,
            ).unwrap_or_default();
            set_f(cmb_lng);

            SendMessageW(cmb_lng, CB_ADDSTRING, WPARAM(0), LPARAM(to_w("English").as_ptr() as isize));
            SendMessageW(cmb_lng, CB_ADDSTRING, WPARAM(0), LPARAM(to_w("Espa\u{00f1}ol").as_ptr() as isize));
            SendMessageW(cmb_lng, CB_SETCURSEL, WPARAM(if cfg.language == "es" { 1 } else { 0 }), LPARAM(0));

            // Autostart checkbox
            let chk_auto = CreateWindowExW(
                Default::default(), w!("BUTTON"), PCWSTR(to_w(i18n.autostart_enable()).as_ptr()),
                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_AUTOCHECKBOX),
                230, 454, 210, 22, hwnd, HMENU(ID_CHK_AUTOSTART as *mut _), instance, None,
            ).unwrap_or_default();
            set_f(chk_auto);
            SendMessageW(chk_auto, BM_SETCHECK, WPARAM(if autostart::is_enabled() { BST_CHECKED } else { BST_UNCHECKED }), LPARAM(0));

            // -------------------------------------------------------------
            // Action Buttons
            // -------------------------------------------------------------
            let btn_save = CreateWindowExW(
                Default::default(), w!("BUTTON"), PCWSTR(to_w(i18n.save_btn()).as_ptr()),
                WS_CHILD | WS_VISIBLE,
                205, 502, 125, 32, hwnd, HMENU(ID_BTN_SAVE as *mut _), instance, None,
            ).unwrap_or_default();
            set_fb(btn_save);

            let btn_close = CreateWindowExW(
                Default::default(), w!("BUTTON"), PCWSTR(to_w(i18n.close_btn()).as_ptr()),
                WS_CHILD | WS_VISIBLE,
                340, 502, 108, 32, hwnd, HMENU(ID_BTN_CLOSE as *mut _), instance, None,
            ).unwrap_or_default();
            set_f(btn_close);

            // Store controls
            {
                let mut guard = CONTROLS.lock().unwrap();
                *guard = Some(SettingsControls {
                    chk_overlay: chk_overlay.0 as isize,
                    cmb_theme: cmb_thm.0 as isize,
                    cmb_position: cmb_pos.0 as isize,
                    cmb_size: cmb_sz.0 as isize,
                    chk_sound: chk_snd.0 as isize,
                    cmb_sound: cmb_snd.0 as isize,
                    chk_toast: chk_tst.0 as isize,
                    cmb_lang: cmb_lng.0 as isize,
                    chk_autostart: chk_auto.0 as isize,
                    lbl_caps_val: lbl_c_val.0 as isize,
                    lbl_num_val: lbl_n_val.0 as isize,
                    lbl_scroll_val: lbl_s_val.0 as isize,
                    font_handle: hfont.0 as isize,
                    font_bold_handle: hfont_bold.0 as isize,
                });
            }

            LRESULT(0)
        }

        WM_COMMAND => {
            let cmd_id = (wparam.0 & 0xffff) as usize;
            match cmd_id {
                ID_BTN_TEST_HUD => {
                    let preview_cfg = read_current_dialog_config();
                    overlay::show("Caps Lock", STATE.is_caps_on(), &preview_cfg);
                }
                ID_BTN_TEST_SOUND => {
                    let preview_cfg = read_current_dialog_config();
                    sound::play(&preview_cfg.sound_theme, true);
                }
                ID_BTN_SAVE => {
                    let mut new_cfg = read_current_dialog_config();
                    new_cfg.first_run = false; // Mark first run completed
                    let _ = new_cfg.save();

                    if new_cfg.autostart {
                        let _ = autostart::enable();
                    } else {
                        let _ = autostart::disable();
                    }

                    if let Ok(cb) = ON_SAVE_CALLBACK.lock() {
                        if let Some(f) = cb.as_ref() {
                            f(new_cfg);
                        }
                    }

                    let _ = DestroyWindow(hwnd);
                }
                ID_BTN_CLOSE => {
                    let _ = DestroyWindow(hwnd);
                }
                _ => {}
            }
            LRESULT(0)
        }

        WM_DESTROY => {
            if let Ok(mut guard) = CONTROLS.lock() {
                if let Some(c) = guard.take() {
                    let _ = DeleteObject(HFONT(c.font_handle as *mut _));
                    let _ = DeleteObject(HFONT(c.font_bold_handle as *mut _));
                }
            }
            SETTINGS_HWND.store(0, Ordering::SeqCst);
            PostQuitMessage(0);
            LRESULT(0)
        }

        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn read_current_dialog_config() -> Config {
    let mut cfg = CURRENT_CONFIG.lock().unwrap().clone().unwrap_or_default();
    if let Ok(guard) = CONTROLS.lock() {
        if let Some(c) = guard.as_ref() {
            unsafe {
                cfg.overlay_enabled = SendMessageW(HWND(c.chk_overlay as *mut _), BM_GETCHECK, WPARAM(0), LPARAM(0)).0 as usize == BST_CHECKED;
                cfg.sound_enabled = SendMessageW(HWND(c.chk_sound as *mut _), BM_GETCHECK, WPARAM(0), LPARAM(0)).0 as usize == BST_CHECKED;
                cfg.toast_enabled = SendMessageW(HWND(c.chk_toast as *mut _), BM_GETCHECK, WPARAM(0), LPARAM(0)).0 as usize == BST_CHECKED;
                cfg.autostart = SendMessageW(HWND(c.chk_autostart as *mut _), BM_GETCHECK, WPARAM(0), LPARAM(0)).0 as usize == BST_CHECKED;

                let thm_idx = SendMessageW(HWND(c.cmb_theme as *mut _), CB_GETCURSEL, WPARAM(0), LPARAM(0)).0 as usize;
                let themes = ["CapsNotifyModern", "LenovoClassic"];
                if thm_idx < themes.len() {
                    cfg.overlay_theme = themes[thm_idx].to_string();
                }

                let pos_idx = SendMessageW(HWND(c.cmb_position as *mut _), CB_GETCURSEL, WPARAM(0), LPARAM(0)).0 as usize;
                let positions = ["TopCenter", "TopLeft", "TopRight", "CenterLeft", "Center", "CenterRight", "BottomLeft", "BottomCenter", "BottomRight"];
                if pos_idx < positions.len() {
                    cfg.overlay_position = positions[pos_idx].to_string();
                }

                let sz_idx = SendMessageW(HWND(c.cmb_size as *mut _), CB_GETCURSEL, WPARAM(0), LPARAM(0)).0 as usize;
                let sizes = ["Small", "Medium", "Large"];
                if sz_idx < sizes.len() {
                    cfg.overlay_size = sizes[sz_idx].to_string();
                }

                let snd_idx = SendMessageW(HWND(c.cmb_sound as *mut _), CB_GETCURSEL, WPARAM(0), LPARAM(0)).0 as usize;
                let sound_themes = ["ModernChime", "KeyClick", "WindowsDefault"];
                if snd_idx < sound_themes.len() {
                    cfg.sound_theme = sound_themes[snd_idx].to_string();
                }

                let lang_idx = SendMessageW(HWND(c.cmb_lang as *mut _), CB_GETCURSEL, WPARAM(0), LPARAM(0)).0 as usize;
                cfg.language = if lang_idx == 1 { "es".to_string() } else { "en".to_string() };
            }
        }
    }
    cfg
}
