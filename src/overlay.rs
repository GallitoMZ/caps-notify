use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::Mutex;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateFontW, CreatePen, CreateSolidBrush, DeleteObject, DrawTextW, EndPaint,
    FillRect, FrameRect, SelectObject, SetBkMode, SetTextColor, DT_CENTER, DT_NOPREFIX,
    DT_SINGLELINE, FW_BOLD, FW_SEMIBOLD, PAINTSTRUCT, PS_SOLID, TRANSPARENT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetSystemMetrics, GetWindowRect, KillTimer,
    RegisterClassW, SetLayeredWindowAttributes, SetTimer, SetWindowPos, ShowWindow, CS_HREDRAW,
    CS_VREDRAW, HWND_TOPMOST, LWA_ALPHA, SM_CXSCREEN, SM_CYSCREEN, SWP_NOACTIVATE,
    SWP_SHOWWINDOW, SW_HIDE, SW_SHOWNOACTIVATE, WM_DESTROY, WM_ERASEBKGND, WM_PAINT, WM_TIMER,
    WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_EX_TRANSPARENT, WS_POPUP,
};

const OVERLAY_CLASS_NAME: PCWSTR = w!("CapsNotifyOverlayClass");
const TIMER_ID: usize = 1001;

static OVERLAY_HWND: AtomicIsize = AtomicIsize::new(0);

struct OverlayData {
    key_name: String,
    enabled: bool,
}

static OVERLAY_DATA: Mutex<OverlayData> = Mutex::new(OverlayData {
    key_name: String::new(),
    enabled: false,
});

/// Displays a floating HUD overlay indicator
pub fn show(key_name: &str, enabled: bool, position: &str, duration_ms: u32) {
    {
        if let Ok(mut data) = OVERLAY_DATA.lock() {
            data.key_name = key_name.to_string();
            data.enabled = enabled;
        }
    }

    let hwnd = get_or_create_overlay();
    if hwnd.0.is_null() {
        return;
    }

    let width = 300;
    let height = 76;

    let (screen_w, screen_h) = unsafe {
        (
            GetSystemMetrics(SM_CXSCREEN),
            GetSystemMetrics(SM_CYSCREEN),
        )
    };

    let (x, y) = match position {
        "TopRight" => (screen_w - width - 40, 50),
        "BottomRight" => (screen_w - width - 40, screen_h - height - 80),
        "BottomCenter" => ((screen_w - width) / 2, screen_h - height - 80),
        "Center" => ((screen_w - width) / 2, (screen_h - height) / 2),
        _ => ((screen_w - width) / 2, 50), // Default: TopCenter
    };

    unsafe {
        let _ = SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            x,
            y,
            width,
            height,
            SWP_SHOWWINDOW | SWP_NOACTIVATE,
        );

        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);

        // Reset display timer
        let _ = KillTimer(hwnd, TIMER_ID);
        SetTimer(hwnd, TIMER_ID, duration_ms, None);
    }
}

fn get_or_create_overlay() -> HWND {
    let raw = OVERLAY_HWND.load(Ordering::SeqCst);
    if raw != 0 {
        return HWND(raw as *mut _);
    }

    unsafe {
        let instance = GetModuleHandleW(None).unwrap_or_default();
        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(overlay_wndproc),
            hInstance: instance.into(),
            lpszClassName: OVERLAY_CLASS_NAME,
            ..Default::default()
        };
        let _ = RegisterClassW(&wc);

        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TRANSPARENT,
            OVERLAY_CLASS_NAME,
            w!("CapsNotifyHUD"),
            WS_POPUP,
            0,
            0,
            300,
            76,
            None,
            None,
            instance,
            None,
        ).unwrap_or_default();

        if !hwnd.0.is_null() {
            // Set 90% opacity (230 / 255)
            let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 230, LWA_ALPHA);
            OVERLAY_HWND.store(hwnd.0 as isize, Ordering::SeqCst);
        }

        hwnd
    }
}

pub fn destroy() {
    let raw = OVERLAY_HWND.swap(0, Ordering::SeqCst);
    if raw != 0 {
        unsafe {
            let _ = DestroyWindow(HWND(raw as *mut _));
        }
    }
}

unsafe extern "system" fn overlay_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_TIMER => {
            if wparam.0 == TIMER_ID {
                let _ = KillTimer(hwnd, TIMER_ID);
                let _ = ShowWindow(hwnd, SW_HIDE);
            }
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);

            let mut rect = RECT::default();
            let _ = GetWindowRect(hwnd, &mut rect);
            let w = rect.right - rect.left;
            let h = rect.bottom - rect.top;
            let local_rect = RECT {
                left: 0,
                top: 0,
                right: w,
                bottom: h,
            };

            // Background dark slate brush
            let bg_brush = CreateSolidBrush(COLORREF(0x00231915)); // Dark BGR: #151923
            FillRect(hdc, &local_rect, bg_brush);
            let _ = DeleteObject(bg_brush);

            // Subtle border
            let border_pen = CreatePen(PS_SOLID, 1, COLORREF(0x003D332B));
            let old_pen = SelectObject(hdc, border_pen);
            let border_rect = local_rect;
            let _ = FrameRect(hdc, &border_rect, CreateSolidBrush(COLORREF(0x004A3C33)));
            SelectObject(hdc, old_pen);
            let _ = DeleteObject(border_pen);

            let (key_title, enabled) = {
                if let Ok(data) = OVERLAY_DATA.lock() {
                    (data.key_name.clone(), data.enabled)
                } else {
                    (String::new(), false)
                }
            };

            let _ = SetBkMode(hdc, TRANSPARENT);

            // Title Font
            let font_name = w!("Segoe UI");
            let hfont_title = CreateFontW(
                22,
                0,
                0,
                0,
                FW_BOLD.0 as i32,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                font_name,
            );
            let old_font = SelectObject(hdc, hfont_title);

            let mut title_rect = RECT {
                left: 16,
                top: 10,
                right: w - 16,
                bottom: 38,
            };

            SetTextColor(hdc, COLORREF(0x00FFFFFF));
            let wide_title: Vec<u16> = key_title.encode_utf16().collect();
            DrawTextW(
                hdc,
                &mut wide_title.clone(),
                &mut title_rect,
                DT_CENTER | DT_SINGLELINE | DT_NOPREFIX,
            );

            // Subtitle Font (State)
            let hfont_sub = CreateFontW(
                18,
                0,
                0,
                0,
                FW_SEMIBOLD.0 as i32,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                font_name,
            );
            SelectObject(hdc, hfont_sub);

            let status_text = if enabled { "ACTIVATED" } else { "DEACTIVATED" };
            // Emerald green (#2ED573) or Muted Gray (#8892B0) in BGR
            let status_color = if enabled {
                COLORREF(0x0073D52E)
            } else {
                COLORREF(0x008A8580)
            };
            SetTextColor(hdc, status_color);

            let mut sub_rect = RECT {
                left: 16,
                top: 40,
                right: w - 16,
                bottom: 66,
            };
            let wide_sub: Vec<u16> = status_text.encode_utf16().collect();
            DrawTextW(
                hdc,
                &mut wide_sub.clone(),
                &mut sub_rect,
                DT_CENTER | DT_SINGLELINE | DT_NOPREFIX,
            );

            SelectObject(hdc, old_font);
            let _ = DeleteObject(hfont_title);
            let _ = DeleteObject(hfont_sub);

            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_DESTROY => {
            OVERLAY_HWND.store(0, Ordering::SeqCst);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
