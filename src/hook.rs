use std::sync::atomic::{AtomicIsize, Ordering};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{VK_CAPITAL, VK_NUMLOCK, VK_SCROLL};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, PostMessageW, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK,
    KBDLLHOOKSTRUCT, WH_KEYBOARD_LL, WM_APP, WM_KEYUP, WM_SYSKEYUP,
};

pub const WM_APP_HOOK: u32 = WM_APP + 1;

static TARGET_HWND: AtomicIsize = AtomicIsize::new(0);

/// Installs the low-level keyboard hook (WH_KEYBOARD_LL)
pub fn install(hwnd: HWND) -> windows::core::Result<HHOOK> {
    TARGET_HWND.store(hwnd.0 as isize, Ordering::SeqCst);
    unsafe {
        SetWindowsHookExW(
            WH_KEYBOARD_LL,
            Some(hook_proc),
            None,
            0,
        )
    }
}

/// Uninstalls the low-level keyboard hook
pub fn uninstall(h: HHOOK) {
    if !h.0.is_null() {
        unsafe {
            let _ = UnhookWindowsHookEx(h);
        }
    }
    TARGET_HWND.store(0, Ordering::SeqCst);
}

/// Low-level keyboard hook procedure.
/// Must be fast and never block the input thread.
unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 && (wparam.0 as u32 == WM_KEYUP || wparam.0 as u32 == WM_SYSKEYUP) {
        let kbd = *(lparam.0 as *const KBDLLHOOKSTRUCT);
        let vk = kbd.vkCode;

        if vk == VK_CAPITAL.0 as u32 || vk == VK_NUMLOCK.0 as u32 || vk == VK_SCROLL.0 as u32 {
            let hwnd_raw = TARGET_HWND.load(Ordering::Relaxed);
            if hwnd_raw != 0 {
                // Asynchronously post message to hidden window loop to defer processing
                let _ = PostMessageW(
                    HWND(hwnd_raw as *mut core::ffi::c_void),
                    WM_APP_HOOK,
                    WPARAM(vk as usize),
                    LPARAM(0),
                );
            }
        }
    }

    CallNextHookEx(None, code, wparam, lparam)
}
