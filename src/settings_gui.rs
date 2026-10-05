use crate::autostart;
use crate::config::Config;
use crate::overlay;
use crate::sound;
use crate::state::STATE;

use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::Mutex;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{CreateFontW, FW_BOLD, FW_NORMAL};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
    GetSystemMetrics, IsDialogMessageW, PostQuitMessage, RegisterClassW, SendMessageW,
    SetForegroundWindow, SetWindowTextW, ShowWindow, TranslateMessage, CS_HREDRAW, CS_VREDRAW,
    HMENU, MSG, SM_CXSCREEN, SM_CYSCREEN, SW_SHOW, WINDOW_STYLE, WM_COMMAND, WM_CREATE,
    WM_DESTROY, WNDCLASSW, WS_CAPTION, WS_CHILD, WS_MINIMIZEBOX, WS_OVERLAPPED, WS_SYSMENU,
    WS_VISIBLE,
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
const ID_CMB_POSITION: usize = 102;
const ID_CMB_THEME: usize = 103;
const ID_CMB_SIZE: usize = 104;
const ID_BTN_TEST_HUD: usize = 105;

const ID_CHK_SOUND: usize = 201;
const ID_CMB_SOUND: usize = 202;
const ID_BTN_TEST_SOUND: usize = 203;
const ID_CHK_TOAST: usize = 204;

const ID_CHK_AUTOSTART: usize = 301;
const ID_CHK_START_OPEN: usize = 302;

const ID_BTN_SAVE: usize = 401;
const ID_BTN_CLOSE: usize = 402;

static SETTINGS_HWND: AtomicIsize = AtomicIsize::new(0);
static CURRENT_CONFIG: Mutex<Option<Config>> = Mutex::new(None);
static ON_SAVE_CALLBACK: Mutex<Option<Box<dyn Fn(Config) + Send + Sync>>> = Mutex::new(None);

// Control HWNDs stored as raw isize to implement Send + Sync
struct SettingsControls {
    chk_overlay: isize,
    cmb_position: isize,
    cmb_theme: isize,
    cmb_size: isize,
    chk_sound: isize,
    cmb_sound: isize,
    chk_toast: isize,
    chk_autostart: isize,
    chk_start_open: isize,
    lbl_caps_status: isize,
    lbl_num_status: isize,
    lbl_scroll_status: isize,
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
        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(settings_wndproc),
            hInstance: instance.into(),
            lpszClassName: SETTINGS_CLASS_NAME,
            hbrBackground: windows::Win32::Graphics::Gdi::HBRUSH((windows::Win32::Graphics::Gdi::COLOR_BTNFACE.0 + 1) as *mut _),
            ..Default::default()
        };
        let _ = RegisterClassW(&wc);

        let win_w = 460;
        let win_h = 560;
        let screen_w = GetSystemMetrics(SM_CXSCREEN);
        let screen_h = GetSystemMetrics(SM_CYSCREEN);
        let x = (screen_w - win_w) / 2;
        let y = (screen_h - win_h) / 2;

        let hwnd = CreateWindowExW(
            Default::default(),
            SETTINGS_CLASS_NAME,
            w!("Caps Notify — Configuraci\u{00f3}n"),
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
            unsafe {
                let caps_txt = if STATE.is_caps_on() {
                    w!("Bloq May\u{00fa}s: [ ACTIVADO ]")
                } else {
                    w!("Bloq May\u{00fa}s: [ DESACTIVADO ]")
                };
                let num_txt = if STATE.is_num_on() {
                    w!("Bloq Num: [ ACTIVADO ]")
                } else {
                    w!("Bloq Num: [ DESACTIVADO ]")
                };
                let scroll_txt = if STATE.is_scroll_on() {
                    w!("Bloq Despl: [ ACTIVADO ]")
                } else {
                    w!("Bloq Despl: [ DESACTIVADO ]")
                };

                let _ = SetWindowTextW(HWND(c.lbl_caps_status as *mut _), caps_txt);
                let _ = SetWindowTextW(HWND(c.lbl_num_status as *mut _), num_txt);
                let _ = SetWindowTextW(HWND(c.lbl_scroll_status as *mut _), scroll_txt);
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

            let _hfont = CreateFontW(
                16, 0, 0, 0, FW_NORMAL.0 as i32, 0, 0, 0, 0, 0, 0, 0, 0, w!("Segoe UI"),
            );
            let _hfont_bold = CreateFontW(
                16, 0, 0, 0, FW_BOLD.0 as i32, 0, 0, 0, 0, 0, 0, 0, 0, w!("Segoe UI"),
            );

            // Group 1: Live Status
            let _ = CreateWindowExW(
                Default::default(), w!("BUTTON"), w!(" Estado en Tiempo Real "),
                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_GROUPBOX),
                16, 12, 412, 68, hwnd, None, instance, None,
            );

            let lbl_caps = CreateWindowExW(
                Default::default(), w!("STATIC"),
                if STATE.is_caps_on() { w!("Bloq May\u{00fa}s: [ ACTIVADO ]") } else { w!("Bloq May\u{00fa}s: [ DESACTIVADO ]") },
                WS_CHILD | WS_VISIBLE, 28, 32, 180, 20, hwnd, None, instance, None,
            ).unwrap_or_default();

            let lbl_num = CreateWindowExW(
                Default::default(), w!("STATIC"),
                if STATE.is_num_on() { w!("Bloq Num: [ ACTIVADO ]") } else { w!("Bloq Num: [ DESACTIVADO ]") },
                WS_CHILD | WS_VISIBLE, 28, 52, 180, 20, hwnd, None, instance, None,
            ).unwrap_or_default();

            let lbl_scroll = CreateWindowExW(
                Default::default(), w!("STATIC"),
                if STATE.is_scroll_on() { w!("Bloq Despl: [ ACTIVADO ]") } else { w!("Bloq Despl: [ DESACTIVADO ]") },
                WS_CHILD | WS_VISIBLE, 230, 32, 180, 20, hwnd, None, instance, None,
            ).unwrap_or_default();

            // Group 2: HUD Overlay Options
            let _ = CreateWindowExW(
                Default::default(), w!("BUTTON"), w!(" HUD Flotante (On-Screen Display) "),
                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_GROUPBOX),
                16, 90, 412, 175, hwnd, None, instance, None,
            );

            let chk_overlay = CreateWindowExW(
                Default::default(), w!("BUTTON"), w!("Activar HUD flotante en pantalla"),
                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_AUTOCHECKBOX),
                28, 114, 260, 22, hwnd, HMENU(ID_CHK_OVERLAY as *mut _), instance, None,
            ).unwrap_or_default();
            SendMessageW(chk_overlay, BM_SETCHECK, WPARAM(if cfg.overlay_enabled { BST_CHECKED } else { BST_UNCHECKED }), LPARAM(0));

            // Position Combo
            let _ = CreateWindowExW(Default::default(), w!("STATIC"), w!("Posici\u{00f3}n:"), WS_CHILD | WS_VISIBLE, 28, 146, 70, 20, hwnd, None, instance, None);
            let cmb_pos = CreateWindowExW(
                Default::default(), w!("COMBOBOX"), None,
                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(CBS_DROPDOWNLIST),
                105, 142, 170, 200, hwnd, HMENU(ID_CMB_POSITION as *mut _), instance, None,
            ).unwrap_or_default();

            let positions = [
                ("TopCenter", "Arriba Centro (Por defecto)"),
                ("TopLeft", "Arriba Izquierda"),
                ("TopRight", "Arriba Derecha"),
                ("CenterLeft", "Centro Izquierda"),
                ("Center", "Centro Total"),
                ("CenterRight", "Centro Derecha"),
                ("BottomLeft", "Abajo Izquierda"),
                ("BottomCenter", "Abajo Centro"),
                ("BottomRight", "Abajo Derecha"),
            ];

            let mut selected_pos_idx = 0;
            for (idx, (val, label)) in positions.iter().enumerate() {
                let wide: Vec<u16> = label.encode_utf16().chain(std::iter::once(0)).collect();
                SendMessageW(cmb_pos, CB_ADDSTRING, WPARAM(0), LPARAM(wide.as_ptr() as isize));
                if *val == cfg.overlay_position {
                    selected_pos_idx = idx;
                }
            }
            SendMessageW(cmb_pos, CB_SETCURSEL, WPARAM(selected_pos_idx), LPARAM(0));

            // Theme Combo
            let _ = CreateWindowExW(Default::default(), w!("STATIC"), w!("Tema:"), WS_CHILD | WS_VISIBLE, 28, 178, 70, 20, hwnd, None, instance, None);
            let cmb_thm = CreateWindowExW(
                Default::default(), w!("COMBOBOX"), None,
                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(CBS_DROPDOWNLIST),
                105, 174, 170, 150, hwnd, HMENU(ID_CMB_THEME as *mut _), instance, None,
            ).unwrap_or_default();

            let themes = [
                ("LenovoKeycap", "Lenovo Keycap (Original)"),
                ("AccentColor", "Acento Verde / Gris"),
            ];
            let mut selected_thm_idx = 0;
            for (idx, (val, label)) in themes.iter().enumerate() {
                let wide: Vec<u16> = label.encode_utf16().chain(std::iter::once(0)).collect();
                SendMessageW(cmb_thm, CB_ADDSTRING, WPARAM(0), LPARAM(wide.as_ptr() as isize));
                if *val == cfg.overlay_theme {
                    selected_thm_idx = idx;
                }
            }
            SendMessageW(cmb_thm, CB_SETCURSEL, WPARAM(selected_thm_idx), LPARAM(0));

            // Size Combo
            let _ = CreateWindowExW(Default::default(), w!("STATIC"), w!("Tama\u{00f1}o:"), WS_CHILD | WS_VISIBLE, 28, 210, 70, 20, hwnd, None, instance, None);
            let cmb_sz = CreateWindowExW(
                Default::default(), w!("COMBOBOX"), None,
                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(CBS_DROPDOWNLIST),
                105, 206, 170, 120, hwnd, HMENU(ID_CMB_SIZE as *mut _), instance, None,
            ).unwrap_or_default();

            let sizes = [
                ("Small", "Compacto (104 px)"),
                ("Medium", "Normal (128 px)"),
                ("Large", "Grande (156 px)"),
            ];
            let mut selected_sz_idx = 1;
            for (idx, (val, label)) in sizes.iter().enumerate() {
                let wide: Vec<u16> = label.encode_utf16().chain(std::iter::once(0)).collect();
                SendMessageW(cmb_sz, CB_ADDSTRING, WPARAM(0), LPARAM(wide.as_ptr() as isize));
                if *val == cfg.overlay_size {
                    selected_sz_idx = idx;
                }
            }
            SendMessageW(cmb_sz, CB_SETCURSEL, WPARAM(selected_sz_idx), LPARAM(0));

            // Test HUD Button
            let _ = CreateWindowExW(
                Default::default(), w!("BUTTON"), w!("Probar HUD"),
                WS_CHILD | WS_VISIBLE,
                295, 142, 115, 30, hwnd, HMENU(ID_BTN_TEST_HUD as *mut _), instance, None,
            );

            // Group 3: Audio & Notifications
            let _ = CreateWindowExW(
                Default::default(), w!("BUTTON"), w!(" Sonido y Notificaciones "),
                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_GROUPBOX),
                16, 275, 412, 115, hwnd, None, instance, None,
            );

            let chk_snd = CreateWindowExW(
                Default::default(), w!("BUTTON"), w!("Sonido al cambiar tecla"),
                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_AUTOCHECKBOX),
                28, 298, 220, 22, hwnd, HMENU(ID_CHK_SOUND as *mut _), instance, None,
            ).unwrap_or_default();
            SendMessageW(chk_snd, BM_SETCHECK, WPARAM(if cfg.sound_enabled { BST_CHECKED } else { BST_UNCHECKED }), LPARAM(0));

            let cmb_snd = CreateWindowExW(
                Default::default(), w!("COMBOBOX"), None,
                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(CBS_DROPDOWNLIST),
                28, 326, 170, 120, hwnd, HMENU(ID_CMB_SOUND as *mut _), instance, None,
            ).unwrap_or_default();

            let sound_themes = [
                ("ModernChime", "Chime Moderno (Arm\u{00f3}nico)"),
                ("KeyClick", "Clic Mec\u{00e1}nico"),
                ("WindowsDefault", "Sonido de Windows"),
            ];
            let mut selected_snd_idx = 0;
            for (idx, (val, label)) in sound_themes.iter().enumerate() {
                let wide: Vec<u16> = label.encode_utf16().chain(std::iter::once(0)).collect();
                SendMessageW(cmb_snd, CB_ADDSTRING, WPARAM(0), LPARAM(wide.as_ptr() as isize));
                if *val == cfg.sound_theme {
                    selected_snd_idx = idx;
                }
            }
            SendMessageW(cmb_snd, CB_SETCURSEL, WPARAM(selected_snd_idx), LPARAM(0));

            let _ = CreateWindowExW(
                Default::default(), w!("BUTTON"), w!("Probar sonido"),
                WS_CHILD | WS_VISIBLE,
                210, 325, 105, 26, hwnd, HMENU(ID_BTN_TEST_SOUND as *mut _), instance, None,
            );

            let chk_tst = CreateWindowExW(
                Default::default(), w!("BUTTON"), w!("Notificaciones Toast de Windows"),
                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_AUTOCHECKBOX),
                28, 358, 250, 22, hwnd, HMENU(ID_CHK_TOAST as *mut _), instance, None,
            ).unwrap_or_default();
            SendMessageW(chk_tst, BM_SETCHECK, WPARAM(if cfg.toast_enabled { BST_CHECKED } else { BST_UNCHECKED }), LPARAM(0));

            // Group 4: System Options
            let _ = CreateWindowExW(
                Default::default(), w!("BUTTON"), w!(" Opciones de Sistema "),
                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_GROUPBOX),
                16, 400, 412, 70, hwnd, None, instance, None,
            );

            let chk_auto = CreateWindowExW(
                Default::default(), w!("BUTTON"), w!("Iniciar con Windows (Autostart)"),
                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_AUTOCHECKBOX),
                28, 420, 250, 22, hwnd, HMENU(ID_CHK_AUTOSTART as *mut _), instance, None,
            ).unwrap_or_default();
            SendMessageW(chk_auto, BM_SETCHECK, WPARAM(if autostart::is_enabled() { BST_CHECKED } else { BST_UNCHECKED }), LPARAM(0));

            let chk_open = CreateWindowExW(
                Default::default(), w!("BUTTON"), w!("Abrir esta ventana al iniciar la app"),
                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_AUTOCHECKBOX),
                28, 442, 280, 22, hwnd, HMENU(ID_CHK_START_OPEN as *mut _), instance, None,
            ).unwrap_or_default();
            SendMessageW(chk_open, BM_SETCHECK, WPARAM(if cfg.show_settings_on_start { BST_CHECKED } else { BST_UNCHECKED }), LPARAM(0));

            // Bottom Buttons
            let _ = CreateWindowExW(
                Default::default(), w!("BUTTON"), w!("Guardar y Aplicar"),
                WS_CHILD | WS_VISIBLE,
                180, 480, 130, 32, hwnd, HMENU(ID_BTN_SAVE as *mut _), instance, None,
            );

            let _ = CreateWindowExW(
                Default::default(), w!("BUTTON"), w!("Cerrar"),
                WS_CHILD | WS_VISIBLE,
                320, 480, 108, 32, hwnd, HMENU(ID_BTN_CLOSE as *mut _), instance, None,
            );

            // Store controls as isize
            {
                let mut guard = CONTROLS.lock().unwrap();
                *guard = Some(SettingsControls {
                    chk_overlay: chk_overlay.0 as isize,
                    cmb_position: cmb_pos.0 as isize,
                    cmb_theme: cmb_thm.0 as isize,
                    cmb_size: cmb_sz.0 as isize,
                    chk_sound: chk_snd.0 as isize,
                    cmb_sound: cmb_snd.0 as isize,
                    chk_toast: chk_tst.0 as isize,
                    chk_autostart: chk_auto.0 as isize,
                    chk_start_open: chk_open.0 as isize,
                    lbl_caps_status: lbl_caps.0 as isize,
                    lbl_num_status: lbl_num.0 as isize,
                    lbl_scroll_status: lbl_scroll.0 as isize,
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
                    let new_cfg = read_current_dialog_config();
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
                cfg.show_settings_on_start = SendMessageW(HWND(c.chk_start_open as *mut _), BM_GETCHECK, WPARAM(0), LPARAM(0)).0 as usize == BST_CHECKED;

                let pos_idx = SendMessageW(HWND(c.cmb_position as *mut _), CB_GETCURSEL, WPARAM(0), LPARAM(0)).0 as usize;
                let positions = ["TopCenter", "TopLeft", "TopRight", "CenterLeft", "Center", "CenterRight", "BottomLeft", "BottomCenter", "BottomRight"];
                if pos_idx < positions.len() {
                    cfg.overlay_position = positions[pos_idx].to_string();
                }

                let thm_idx = SendMessageW(HWND(c.cmb_theme as *mut _), CB_GETCURSEL, WPARAM(0), LPARAM(0)).0 as usize;
                let themes = ["LenovoKeycap", "AccentColor"];
                if thm_idx < themes.len() {
                    cfg.overlay_theme = themes[thm_idx].to_string();
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
            }
        }
    }
    cfg
}
