#![windows_subsystem = "windows"]

mod autostart;
mod config;
mod hook;
mod notifier;
mod overlay;
mod state;
mod tray;

use config::Config;
use notifier::LockKey;
use state::STATE;
use tray::{
    Tray, ID_TRAY_ABOUT, ID_TRAY_AUTOSTART, ID_TRAY_CONFIG, ID_TRAY_EXIT, ID_TRAY_OVERLAY,
    ID_TRAY_SOUND, ID_TRAY_TOASTS, WM_APP_TRAY,
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
    PostQuitMessage, RegisterClassW, RegisterWindowMessageW, TranslateMessage, HHOOK, MSG,
    SW_SHOW, WM_COMMAND, WM_DESTROY, WM_LBUTTONDBLCLK, WM_RBUTTONUP, WNDCLASSW,
    WS_OVERLAPPEDWINDOW, MessageBoxW, MB_ICONINFORMATION, MB_OK,
};

const WINDOW_CLASS_NAME: PCWSTR = w!("CapsNotifyHiddenWindow");

thread_local! {
    static TRAY_INSTANCE: RefCell<Option<Tray>> = RefCell::new(None);
    static HOOK_HANDLE: RefCell<Option<HHOOK>> = RefCell::new(None);
    static TASKBAR_RESTART_MSG: RefCell<u32> = RefCell::new(0);
}

static APP_CONFIG: Mutex<Option<Config>> = Mutex::new(None);

fn get_config() -> Config {
    if let Ok(guard) = APP_CONFIG.lock() {
        if let Some(cfg) = &*guard {
            return cfg.clone();
        }
    }
    Config::load()
}

fn update_config<F: FnOnce(&mut Config)>(f: F) {
    if let Ok(mut guard) = APP_CONFIG.lock() {
        if let Some(cfg) = &mut *guard {
            f(cfg);
            let _ = cfg.save();
        }
    }
}

fn main() -> windows::core::Result<()> {
    // 1. Initialize COM apartment for WinRT toasts and Shell APIs
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }

    // 2. Load configuration
    let cfg = Config::load();
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

    // 8. Classical Win32 message loop (event-driven, 0% CPU idle)
    unsafe {
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).into() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    // 9. Cleanup
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
        // Explorer restarted: re-create tray icon
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
                let current = (GetKeyState(VK_CAPITAL.0 as i32) & 0x0001) != 0;
                let prev = STATE.caps.swap(current, Ordering::SeqCst);
                if prev != current {
                    TRAY_INSTANCE.with(|t| {
                        if let Some(tray) = t.borrow().as_ref() {
                            tray.update(&STATE, &cfg);
                        }
                    });
                    if cfg.watch_caps {
                        notifier::notify(LockKey::Caps, current, &cfg);
                    }
                }
            } else if vk == VK_NUMLOCK.0 {
                let current = (GetKeyState(VK_NUMLOCK.0 as i32) & 0x0001) != 0;
                let prev = STATE.num.swap(current, Ordering::SeqCst);
                if prev != current {
                    TRAY_INSTANCE.with(|t| {
                        if let Some(tray) = t.borrow().as_ref() {
                            tray.update(&STATE, &cfg);
                        }
                    });
                    if cfg.watch_num {
                        notifier::notify(LockKey::Num, current, &cfg);
                    }
                }
            } else if vk == VK_SCROLL.0 {
                let current = (GetKeyState(VK_SCROLL.0 as i32) & 0x0001) != 0;
                let prev = STATE.scroll.swap(current, Ordering::SeqCst);
                if prev != current {
                    TRAY_INSTANCE.with(|t| {
                        if let Some(tray) = t.borrow().as_ref() {
                            tray.update(&STATE, &cfg);
                        }
                    });
                    if cfg.watch_scroll {
                        notifier::notify(LockKey::Scroll, current, &cfg);
                    }
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
                    // Double click opens config
                    open_config_file();
                }
                _ => {}
            }
            LRESULT(0)
        }

        // Context menu command handlers
        WM_COMMAND => {
            let cmd_id = (wparam.0 & 0xffff) as usize;
            match cmd_id {
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
                }
                ID_TRAY_OVERLAY => {
                    update_config(|c| c.overlay_enabled = !c.overlay_enabled);
                    let cfg = get_config();
                    if cfg.overlay_enabled {
                        overlay::show(
                            "Caps Lock",
                            STATE.is_caps_on(),
                            &cfg.overlay_position,
                            cfg.overlay_duration_ms,
                        );
                    }
                }
                ID_TRAY_CONFIG => {
                    open_config_file();
                }
                ID_TRAY_ABOUT => {
                    let text = w!("Caps Notify v0.1.0\n\nLightweight lock key tray indicator for Windows.\nZero polling, ultra-low resource footprint.\n\nAuthor: GallitoMZ\nLicense: MIT");
                    let title = w!("About Caps Notify");
                    MessageBoxW(hwnd, text, title, MB_ICONINFORMATION | MB_OK);
                }
                ID_TRAY_EXIT => {
                    let _ = DestroyWindow(hwnd);
                }
                _ => {}
            }
            LRESULT(0)
        }

        WM_DESTROY => {
            // Uninstall keyboard hook
            HOOK_HANDLE.with(|h| {
                if let Some(hhook) = h.borrow_mut().take() {
                    hook::uninstall(hhook);
                }
            });

            // Remove tray icon
            TRAY_INSTANCE.with(|t| {
                if let Some(tray) = t.borrow_mut().take() {
                    tray.remove();
                }
            });

            // Destroy overlay window if open
            overlay::destroy();

            PostQuitMessage(0);
            LRESULT(0)
        }

        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
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
