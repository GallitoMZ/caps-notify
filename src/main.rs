#![windows_subsystem = "windows"]

mod autostart;
mod config;
mod hook;
mod i18n;
mod notifier;
mod overlay;
mod settings_gui;
mod sound;
mod state;
mod tray;

use config::Config;
use i18n::I18n;
use notifier::LockKey;
use state::STATE;
use tray::{
    Tray, ID_POS_BOTTOM_CENTER, ID_POS_BOTTOM_LEFT, ID_POS_BOTTOM_RIGHT, ID_POS_CENTER,
    ID_POS_CENTER_LEFT, ID_POS_CENTER_RIGHT, ID_POS_TOP_CENTER, ID_POS_TOP_LEFT,
    ID_POS_TOP_RIGHT, ID_SND_CLICK, ID_SND_MODERN, ID_SND_WIN, ID_THM_LENOVO, ID_THM_MODERN,
    ID_TRAY_ABOUT, ID_TRAY_AUTOSTART, ID_TRAY_CONFIG, ID_TRAY_EXIT, ID_TRAY_OVERLAY,
    ID_TRAY_SETTINGS, ID_TRAY_SOUND, ID_TRAY_TOASTS, WM_APP_TRAY,
};

use std::cell::RefCell;
use std::sync::atomic::Ordering;
use std::sync::Mutex;

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, VK_CAPITAL, VK_NUMLOCK, VK_SCROLL};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
    MessageBoxW, PostQuitMessage, RegisterClassW, RegisterWindowMessageW, TranslateMessage,
    HHOOK, MB_ICONINFORMATION, MB_OK, MSG, SW_SHOW, WM_COMMAND, WM_DESTROY, WM_LBUTTONDBLCLK,
    WM_RBUTTONUP, WNDCLASSW, WS_OVERLAPPEDWINDOW,
};

const WINDOW_CLASS_NAME: PCWSTR = w!("CapsNotifyHiddenWindow");

thread_local! {
    static TRAY_INSTANCE: RefCell<Option<Tray>> = RefCell::new(None);
    static HOOK_HANDLE: RefCell<Option<HHOOK>> = RefCell::new(None);
    static TASKBAR_RESTART_MSG: RefCell<u32> = RefCell::new(0);
}

static APP_CONFIG: Mutex<Option<Config>> = Mutex::new(None);

pub fn get_config() -> Config {
    if let Ok(guard) = APP_CONFIG.lock() {
        if let Some(cfg) = &*guard {
            return cfg.clone();
        }
    }
    Config::load()
}

pub fn update_config<F: FnOnce(&mut Config)>(f: F) {
    if let Ok(mut guard) = APP_CONFIG.lock() {
        if let Some(cfg) = &mut *guard {
            f(cfg);
            let _ = cfg.save();
        }
    }
}

pub fn set_config(cfg: Config) {
    if let Ok(mut guard) = APP_CONFIG.lock() {
        *guard = Some(cfg.clone());
    }
    TRAY_INSTANCE.with(|t| {
        if let Some(tray) = t.borrow().as_ref() {
            tray.update(&STATE, &cfg);
        }
    });
}

fn main() -> windows::core::Result<()> {
    // 1. Initialize COM apartment
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }

    // 2. Load configuration
    let mut cfg = Config::load();
    let is_first_run = cfg.first_run;

    // Check command line arguments for --minimized / --autostart
    let args: Vec<String> = std::env::args().collect();
    let is_minimized = args.iter().any(|a| a == "--minimized" || a == "--autostart");

    {
        let mut guard = APP_CONFIG.lock().unwrap();
        *guard = Some(cfg.clone());
    }

    // 3. Register TaskbarCreated message to survive explorer.exe restarts
    let taskbar_msg = unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) };
    TASKBAR_RESTART_MSG.with(|m| *m.borrow_mut() = taskbar_msg);

    // 4. Register hidden window class
    let instance = unsafe { GetModuleHandleW(None)? };
    let wc = WNDCLASSW {
        lpfnWndProc: Some(wndproc),
        hInstance: instance.into(),
        lpszClassName: WINDOW_CLASS_NAME,
        ..Default::default()
    };
    unsafe {
        let _ = RegisterClassW(&wc);
    }

    // 5. Create hidden message window
    let hwnd = unsafe {
        CreateWindowExW(
            Default::default(),
            WINDOW_CLASS_NAME,
            w!("CapsNotifyHidden"),
            WS_OVERLAPPEDWINDOW,
            0,
            0,
            0,
            0,
            None,
            None,
            instance,
            None,
        )?
    };

    // 6. Read initial state & initialize tray icon
    STATE.read_all_from_system();
    let tray = Tray::new(hwnd, &STATE, &cfg);
    TRAY_INSTANCE.with(|t| *t.borrow_mut() = Some(tray));

    // 7. Install low-level keyboard hook
    if let Ok(hhook) = hook::install(hwnd) {
        HOOK_HANDLE.with(|h| *h.borrow_mut() = Some(hhook));
    }

    // 8. Visual confirmation on startup:
    // Display HUD briefly so the user instantly sees that Caps Notify is running
    if cfg.overlay_enabled {
        overlay::show("Caps Lock", STATE.is_caps_on(), &cfg);
    }

    // Open settings ONLY on the first run ever (not on subsequent boots/launches)
    if is_first_run && !is_minimized {
        cfg.first_run = false;
        let _ = cfg.save();
        settings_gui::open_settings(cfg.clone(), |new_cfg| {
            set_config(new_cfg);
        });
    }

    // 9. Classical Win32 message loop
    unsafe {
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).into() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    // 10. Cleanup
    unsafe {
        CoUninitialize();
    }

    Ok(())
}

unsafe extern "system" fn wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let taskbar_created = TASKBAR_RESTART_MSG.with(|m| *m.borrow());

    if msg == taskbar_created && taskbar_created != 0 {
        let cfg = get_config();
        TRAY_INSTANCE.with(|t| {
            if let Some(tray) = t.borrow().as_ref() {
                tray.register(&STATE, &cfg);
            }
        });
        return LRESULT(0);
    }

    match msg {
        // Keyboard hook notification (WM_APP + 1)
        hook::WM_APP_HOOK => {
            let vk = wparam.0 as u16;
            let cfg = get_config();

            if vk == VK_CAPITAL.0 {
                let prev = STATE.caps.load(Ordering::SeqCst);
                let sys = (GetKeyState(VK_CAPITAL.0 as i32) & 0x0001) != 0;
                let new_state = if sys != prev { sys } else { !prev };
                STATE.caps.store(new_state, Ordering::SeqCst);

                TRAY_INSTANCE.with(|t| {
                    if let Some(tray) = t.borrow().as_ref() {
                        tray.update(&STATE, &cfg);
                    }
                });

                settings_gui::update_live_status();

                if cfg.watch_caps {
                    notifier::notify(LockKey::Caps, new_state, &cfg);
                }
            } else if vk == VK_NUMLOCK.0 {
                let prev = STATE.num.load(Ordering::SeqCst);
                let sys = (GetKeyState(VK_NUMLOCK.0 as i32) & 0x0001) != 0;
                let new_state = if sys != prev { sys } else { !prev };
                STATE.num.store(new_state, Ordering::SeqCst);

                TRAY_INSTANCE.with(|t| {
                    if let Some(tray) = t.borrow().as_ref() {
                        tray.update(&STATE, &cfg);
                    }
                });

                settings_gui::update_live_status();

                if cfg.watch_num {
                    notifier::notify(LockKey::Num, new_state, &cfg);
                }
            } else if vk == VK_SCROLL.0 {
                let prev = STATE.scroll.load(Ordering::SeqCst);
                let sys = (GetKeyState(VK_SCROLL.0 as i32) & 0x0001) != 0;
                let new_state = if sys != prev { sys } else { !prev };
                STATE.scroll.store(new_state, Ordering::SeqCst);

                TRAY_INSTANCE.with(|t| {
                    if let Some(tray) = t.borrow().as_ref() {
                        tray.update(&STATE, &cfg);
                    }
                });

                settings_gui::update_live_status();

                if cfg.watch_scroll {
                    notifier::notify(LockKey::Scroll, new_state, &cfg);
                }
            }

            LRESULT(0)
        }

        // Tray icon callback (WM_APP + 2)
        WM_APP_TRAY => {
            let event = lparam.0 as u32;
            match event {
                WM_RBUTTONUP => {
                    let cfg = get_config();
                    let is_auto = autostart::is_enabled();
                    TRAY_INSTANCE.with(|t| {
                        if let Some(tray) = t.borrow().as_ref() {
                            tray.show_context_menu(&cfg, is_auto);
                        }
                    });
                }
                WM_LBUTTONDBLCLK => {
                    // Double click opens Settings GUI
                    let cfg = get_config();
                    settings_gui::open_settings(cfg, |new_cfg| {
                        set_config(new_cfg);
                    });
                }
                _ => {}
            }
            LRESULT(0)
        }

        // Context menu commands
        WM_COMMAND => {
            let cmd_id = (wparam.0 & 0xffff) as usize;
            match cmd_id {
                ID_TRAY_SETTINGS => {
                    let cfg = get_config();
                    settings_gui::open_settings(cfg, |new_cfg| {
                        set_config(new_cfg);
                    });
                }
                // Themes
                ID_THM_MODERN => set_theme_and_preview("CapsNotifyModern"),
                ID_THM_LENOVO => set_theme_and_preview("LenovoClassic"),

                // Positions
                ID_POS_TOP_CENTER => set_position_and_preview("TopCenter"),
                ID_POS_TOP_LEFT => set_position_and_preview("TopLeft"),
                ID_POS_TOP_RIGHT => set_position_and_preview("TopRight"),
                ID_POS_CENTER_LEFT => set_position_and_preview("CenterLeft"),
                ID_POS_CENTER => set_position_and_preview("Center"),
                ID_POS_CENTER_RIGHT => set_position_and_preview("CenterRight"),
                ID_POS_BOTTOM_LEFT => set_position_and_preview("BottomLeft"),
                ID_POS_BOTTOM_CENTER => set_position_and_preview("BottomCenter"),
                ID_POS_BOTTOM_RIGHT => set_position_and_preview("BottomRight"),

                // Sound Themes
                ID_SND_MODERN => set_sound_theme_and_preview("ModernChime"),
                ID_SND_CLICK => set_sound_theme_and_preview("KeyClick"),
                ID_SND_WIN => set_sound_theme_and_preview("WindowsDefault"),

                ID_TRAY_AUTOSTART => {
                    if autostart::is_enabled() {
                        let _ = autostart::disable();
                        update_config(|c| c.autostart = false);
                    } else {
                        let _ = autostart::enable();
                        update_config(|c| c.autostart = true);
                    }
                }
                ID_TRAY_TOASTS => {
                    update_config(|c| c.toast_enabled = !c.toast_enabled);
                }
                ID_TRAY_SOUND => {
                    update_config(|c| c.sound_enabled = !c.sound_enabled);
                    let cfg = get_config();
                    if cfg.sound_enabled {
                        sound::play(&cfg.sound_theme, true);
                    }
                }
                ID_TRAY_OVERLAY => {
                    update_config(|c| c.overlay_enabled = !c.overlay_enabled);
                    let cfg = get_config();
                    if cfg.overlay_enabled {
                        overlay::show("Caps Lock", STATE.is_caps_on(), &cfg);
                    }
                }
                ID_TRAY_CONFIG => {
                    open_config_file();
                }
                ID_TRAY_ABOUT => {
                    let cfg = get_config();
                    let i18n = I18n::new(&cfg.language);
                    let text: Vec<u16> = i18n.about_body().encode_utf16().chain(std::iter::once(0)).collect();
                    let title: Vec<u16> = i18n.tray_about().encode_utf16().chain(std::iter::once(0)).collect();
                    MessageBoxW(hwnd, PCWSTR(text.as_ptr()), PCWSTR(title.as_ptr()), MB_ICONINFORMATION | MB_OK);
                }
                ID_TRAY_EXIT => {
                    let _ = DestroyWindow(hwnd);
                }
                _ => {}
            }
            LRESULT(0)
        }

        WM_DESTROY => {
            HOOK_HANDLE.with(|h| {
                if let Some(hhook) = h.borrow_mut().take() {
                    hook::uninstall(hhook);
                }
            });

            TRAY_INSTANCE.with(|t| {
                if let Some(tray) = t.borrow_mut().take() {
                    tray.remove();
                }
            });

            overlay::destroy();

            PostQuitMessage(0);
            LRESULT(0)
        }

        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn set_theme_and_preview(theme: &str) {
    update_config(|c| c.overlay_theme = theme.to_string());
    let cfg = get_config();
    overlay::show("Caps Lock", STATE.is_caps_on(), &cfg);
}

fn set_position_and_preview(pos: &str) {
    update_config(|c| c.overlay_position = pos.to_string());
    let cfg = get_config();
    overlay::show("Caps Lock", STATE.is_caps_on(), &cfg);
}

fn set_sound_theme_and_preview(theme: &str) {
    update_config(|c| {
        c.sound_theme = theme.to_string();
        c.sound_enabled = true;
    });
    sound::play(theme, true);
}

fn open_config_file() {
    let path = Config::get_config_path();
    if !path.exists() {
        let _ = Config::default().save();
    }
    let path_str = path.to_string_lossy();
    let wide_path: Vec<u16> = path_str.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        let _ = ShellExecuteW(
            None,
            w!("open"),
            PCWSTR(wide_path.as_ptr()),
            None,
            None,
            SW_SHOW,
        );
    }
}
