use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::Mutex;
use crate::config::Config;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, CreateFontW, DeleteDC, DeleteObject,
    DrawTextW, SelectObject, SetBkMode, SetTextColor, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    DIB_RGB_COLORS, DT_CENTER, DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, FW_BOLD, FW_SEMIBOLD,
    TRANSPARENT, AC_SRC_ALPHA, AC_SRC_OVER, BLENDFUNCTION,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetSystemMetrics, KillTimer, RegisterClassW,
    SetTimer, ShowWindow, UpdateLayeredWindow, CS_HREDRAW, CS_VREDRAW, SM_CXSCREEN, SM_CYSCREEN,
    SW_HIDE, SW_SHOWNOACTIVATE, ULW_ALPHA, WM_DESTROY, WM_TIMER, WNDCLASSW, WS_EX_LAYERED,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};

const OVERLAY_CLASS_NAME: PCWSTR = w!("CapsNotifyHUDLayered");
const TIMER_ID: usize = 1001;

static OVERLAY_HWND: AtomicIsize = AtomicIsize::new(0);

#[derive(Clone)]
struct HudDisplayState {
    key_type: String, // "Caps", "Num", "Scroll"
    enabled: bool,
    theme: String,
}

static HUD_STATE: Mutex<HudDisplayState> = Mutex::new(HudDisplayState {
    key_type: String::new(),
    enabled: false,
    theme: String::new(),
});

/// Displays the floating HUD overlay indicator
pub fn show(key_name: &str, enabled: bool, cfg: &Config) {
    let key_type = if key_name.contains("Caps") {
        "Caps"
    } else if key_name.contains("Num") {
        "Num"
    } else {
        "Scroll"
    };

    {
        if let Ok(mut st) = HUD_STATE.lock() {
            st.key_type = key_type.to_string();
            st.enabled = enabled;
            st.theme = cfg.overlay_theme.clone();
        }
    }

    let hwnd = get_or_create_overlay();
    if hwnd.0.is_null() {
        return;
    }

    let (win_w, win_h) = match cfg.overlay_theme.as_str() {
        "CyberMinimal" => (176, 64),
        "DynamicIsland" => (160, 68),
        "NeumorphicKey" => (130, 130),
        _ => (144, 144), // CapsNotifyModern
    };

    let (screen_w, screen_h) = unsafe {
        (
            GetSystemMetrics(SM_CXSCREEN),
            GetSystemMetrics(SM_CYSCREEN),
        )
    };

    let margin_x = 45;
    let margin_y = 55;
    let margin_bottom = 85;

    let (x, y) = match cfg.overlay_position.as_str() {
        "TopLeft" => (margin_x, margin_y),
        "TopRight" => (screen_w - win_w - margin_x, margin_y),
        "CenterLeft" => (margin_x, (screen_h - win_h) / 2),
        "Center" => ((screen_w - win_w) / 2, (screen_h - win_h) / 2),
        "CenterRight" => (screen_w - win_w - margin_x, (screen_h - win_h) / 2),
        "BottomLeft" => (margin_x, screen_h - win_h - margin_bottom),
        "BottomCenter" => ((screen_w - win_w) / 2, screen_h - win_h - margin_bottom),
        "BottomRight" => (screen_w - win_w - margin_x, screen_h - win_h - margin_bottom),
        _ => ((screen_w - win_w) / 2, margin_y), // Default: TopCenter
    };

    match cfg.overlay_theme.as_str() {
        "CyberMinimal" => render_cyber_minimal(hwnd, x, y, win_w, win_h, key_type, enabled),
        "NeumorphicKey" => render_neumorphic_key(hwnd, x, y, win_w, win_h, key_type, enabled),
        "DynamicIsland" => render_dynamic_island(hwnd, x, y, win_w, win_h, key_type, enabled),
        _ => render_caps_notify_modern(hwnd, x, y, win_w, win_h, key_type, enabled),
    }

    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        let _ = KillTimer(hwnd, TIMER_ID);
        SetTimer(hwnd, TIMER_ID, cfg.overlay_duration_ms, None);
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
            200,
            160,
            None,
            None,
            instance,
            None,
        ).unwrap_or_default();

        if !hwnd.0.is_null() {
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
        WM_DESTROY => {
            OVERLAY_HWND.store(0, Ordering::SeqCst);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

// =========================================================================
// THEME 1: CapsNotifyModern — Refined, elegant glass card (DEFAULT)
// =========================================================================
fn render_caps_notify_modern(
    hwnd: HWND,
    x: i32,
    y: i32,
    w_px: i32,
    h_px: i32,
    key_type: &str,
    enabled: bool,
) {
    let w = w_px as usize;
    let h = h_px as usize;

    unsafe {
        let hdc_screen = windows::Win32::Graphics::Gdi::GetDC(None);
        let hdc_mem = CreateCompatibleDC(hdc_screen);

        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w_px,
                biHeight: -h_px,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut p_bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let hbmp = CreateDIBSection(
            hdc_mem, &bmi, DIB_RGB_COLORS, &mut p_bits, None, 0,
        ).unwrap_or_default();

        if hbmp.0.is_null() || p_bits.is_null() {
            let _ = DeleteDC(hdc_mem);
            windows::Win32::Graphics::Gdi::ReleaseDC(None, hdc_screen);
            return;
        }

        let old_bmp = SelectObject(hdc_mem, hbmp);
        let pixels = std::slice::from_raw_parts_mut(p_bits as *mut u8, w * h * 4);

        // 1. Dark, opaque obsidian background (alpha = 252 for zero bleed-through)
        let bg_radius = 24.0f32;
        draw_base_rounded_background(pixels, w, h, bg_radius, 252.0, 13, 17, 23);

        // 2. Single, continuous outer border (zero inner lines, zero superimposed artifacts)
        let (border_r, border_g, border_b) = if enabled {
            (45u8, 212u8, 191u8) // Soft luminous Teal/Cyan #2DD4BF
        } else {
            (51u8, 65u8, 85u8) // Slate 700 #334155
        };
        draw_round_rect_stroke(
            pixels, w, h,
            1.5, 1.5,
            (w_px as f32) - 1.5, (h_px as f32) - 1.5,
            bg_radius - 1.0,
            if enabled { 1.6 } else { 1.2 },
            border_r, border_g, border_b,
        );

        let _ = SetBkMode(hdc_mem, TRANSPARENT);

        // 3. Top Header: Key Name (Clean Segoe UI Semibold)
        let header_text = match key_type {
            "Caps" => "CAPS LOCK",
            "Num" => "NUM LOCK",
            _ => "SCROLL LOCK",
        };
        let font_hdr = CreateFontW(
            12, 0, 0, 0, FW_SEMIBOLD.0 as i32,
            0, 0, 0, 0, 0, 0, 0, 0, w!("Segoe UI"),
        );
        let old_font_hdr = SelectObject(hdc_mem, font_hdr);
        let hdr_color = if enabled {
            COLORREF(0x2D | (0xD4 << 8) | (0xBF << 16)) // Soft Teal #2DD4BF
        } else {
            COLORREF(148 | (163 << 8) | (184 << 16)) // Slate-400 #94A3B8
        };
        SetTextColor(hdc_mem, hdr_color);

        let mut rect_hdr = RECT {
            left: 8,
            top: 15,
            right: w_px - 8,
            bottom: 31,
        };
        let wide_hdr: Vec<u16> = header_text.encode_utf16().collect();
        DrawTextW(hdc_mem, &mut wide_hdr.clone(), &mut rect_hdr, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        SelectObject(hdc_mem, old_font_hdr);
        let _ = DeleteObject(font_hdr);

        // 4. Center Hero Typography: "ABC" (ON) / "abc" (OFF); "123"; "SCR"
        let (center_text, font_size, is_bold) = match key_type {
            "Caps" => {
                if enabled {
                    ("ABC", 38, true)
                } else {
                    ("abc", 34, false)
                }
            }
            "Num" => {
                if enabled {
                    ("123", 38, true)
                } else {
                    ("123", 36, false)
                }
            }
            _ => {
                if enabled {
                    ("SCR", 32, true)
                } else {
                    ("SCR", 32, false)
                }
            }
        };

        let font_title = CreateFontW(
            font_size, 0, 0, 0,
            if is_bold { FW_BOLD.0 as i32 } else { FW_SEMIBOLD.0 as i32 },
            0, 0, 0, 0, 0, 0, 0, 0, w!("Segoe UI"),
        );
        let old_font_title = SelectObject(hdc_mem, font_title);
        let title_color = if enabled {
            COLORREF(0x00FFFFFF) // Pure White
        } else {
            COLORREF(148 | (163 << 8) | (184 << 16)) // Soft Slate #94A3B8
        };
        SetTextColor(hdc_mem, title_color);

        let mut rect_title = RECT {
            left: 8,
            top: 36,
            right: w_px - 8,
            bottom: 96,
        };
        let wide_title: Vec<u16> = center_text.encode_utf16().collect();
        DrawTextW(hdc_mem, &mut wide_title.clone(), &mut rect_title, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        SelectObject(hdc_mem, old_font_title);
        let _ = DeleteObject(font_title);

        // Diagonal slash through 123 / SCR when OFF
        if !enabled && (key_type == "Num" || key_type == "Scroll") {
            let slash_x1 = 44.0f32;
            let slash_y1 = 86.0f32;
            let slash_x2 = 100.0f32;
            let slash_y2 = 48.0f32;
            draw_line(pixels, w, h, slash_x1, slash_y1, slash_x2, slash_y2, 3.4, 239, 68, 68); // Vivid Red/Coral #EF4444
        }

        // 5. Bottom Status Capsule Pill ("[ ● ON ]" / "[ ○ OFF ]")
        let pill_w = 66.0f32;
        let pill_h = 24.0f32;
        let pill_x = ((w_px as f32) - pill_w) / 2.0;
        let pill_y = 104.0f32;
        let pill_rad = pill_h / 2.0;

        if enabled {
            draw_solid_round_rect(pixels, w, h, pill_x, pill_y, pill_x + pill_w, pill_y + pill_h, pill_rad, 45, 212, 191, 38);
            draw_round_rect_stroke(pixels, w, h, pill_x, pill_y, pill_x + pill_w, pill_y + pill_h, pill_rad, 1.2, 45, 212, 191);
        } else {
            draw_solid_round_rect(pixels, w, h, pill_x, pill_y, pill_x + pill_w, pill_y + pill_h, pill_rad, 71, 85, 105, 30);
            draw_round_rect_stroke(pixels, w, h, pill_x, pill_y, pill_x + pill_w, pill_y + pill_h, pill_rad, 1.0, 71, 85, 105);
        }

        let status_str = if enabled { "● ON" } else { "○ OFF" };
        let font_sub = CreateFontW(
            12, 0, 0, 0,
            if enabled { FW_BOLD.0 as i32 } else { FW_SEMIBOLD.0 as i32 },
            0, 0, 0, 0, 0, 0, 0, 0, w!("Segoe UI"),
        );
        let old_font_sub = SelectObject(hdc_mem, font_sub);
        let sub_color = if enabled {
            COLORREF(0x00FFFFFF)
        } else {
            COLORREF(148 | (163 << 8) | (184 << 16))
        };
        SetTextColor(hdc_mem, sub_color);

        let mut rect_pill = RECT {
            left: pill_x as i32,
            top: pill_y as i32,
            right: (pill_x + pill_w) as i32,
            bottom: (pill_y + pill_h) as i32,
        };
        let wide_sub: Vec<u16> = status_str.encode_utf16().collect();
        DrawTextW(hdc_mem, &mut wide_sub.clone(), &mut rect_pill, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        SelectObject(hdc_mem, old_font_sub);
        let _ = DeleteObject(font_sub);

        fix_gdi_text_alpha(pixels, w, h, bg_radius);

        commit_layered_window(hwnd, x, y, w_px, h_px, hdc_mem, old_bmp, hbmp, hdc_screen);
    }
}

// =========================================================================
// THEME 2: CyberMinimal — Sleek compact tech pill
// =========================================================================
fn render_cyber_minimal(
    hwnd: HWND,
    x: i32,
    y: i32,
    w_px: i32,
    h_px: i32,
    key_type: &str,
    enabled: bool,
) {
    let w = w_px as usize;
    let h = h_px as usize;

    unsafe {
        let hdc_screen = windows::Win32::Graphics::Gdi::GetDC(None);
        let hdc_mem = CreateCompatibleDC(hdc_screen);
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w_px,
                biHeight: -h_px,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut p_bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let hbmp = CreateDIBSection(hdc_mem, &bmi, DIB_RGB_COLORS, &mut p_bits, None, 0).unwrap_or_default();
        if hbmp.0.is_null() || p_bits.is_null() {
            let _ = DeleteDC(hdc_mem);
            windows::Win32::Graphics::Gdi::ReleaseDC(None, hdc_screen);
            return;
        }
        let old_bmp = SelectObject(hdc_mem, hbmp);
        let pixels = std::slice::from_raw_parts_mut(p_bits as *mut u8, w * h * 4);

        // Background: Compact capsule (radius 18px)
        let rad = (h_px as f32) * 0.28;
        draw_base_rounded_background(pixels, w, h, rad, 252.0, 10, 12, 16);

        let (border_r, border_g, border_b) = if enabled { (56u8, 189u8, 248u8) } else { (51u8, 65u8, 85u8) };
        draw_round_rect_stroke(pixels, w, h, 1.5, 1.5, (w_px as f32) - 1.5, (h_px as f32) - 1.5, rad - 1.0, 1.4, border_r, border_g, border_b);

        // Left Icon
        let icon_x = 24.0f32;
        let icon_y = (h_px as f32) * 0.5;
        match key_type {
            "Caps" => {
                draw_line(pixels, w, h, icon_x - 7.0, icon_y + 4.0, icon_x, icon_y - 4.0, 2.8, border_r, border_g, border_b);
                draw_line(pixels, w, h, icon_x, icon_y - 4.0, icon_x + 7.0, icon_y + 4.0, 2.8, border_r, border_g, border_b);
            }
            "Num" => {
                draw_solid_circle(pixels, w, h, icon_x, icon_y, 4.5, border_r, border_g, border_b);
            }
            _ => {
                draw_line(pixels, w, h, icon_x, icon_y - 6.0, icon_x, icon_y + 6.0, 2.5, border_r, border_g, border_b);
            }
        }

        // Center Text
        let txt = match key_type {
            "Caps" => if enabled { "ABC" } else { "abc" },
            "Num" => "123",
            _ => if enabled { "SCR" } else { "scr" },
        };

        let font = CreateFontW(22, 0, 0, 0, FW_BOLD.0 as i32, 0, 0, 0, 0, 0, 0, 0, 0, w!("Segoe UI"));
        let old_f = SelectObject(hdc_mem, font);
        let _ = SetBkMode(hdc_mem, TRANSPARENT);
        SetTextColor(hdc_mem, COLORREF(0x00FFFFFF));

        let mut rect_txt = RECT { left: 46, top: 12, right: 104, bottom: h_px - 12 };
        let wide: Vec<u16> = txt.encode_utf16().collect();
        DrawTextW(hdc_mem, &mut wide.clone(), &mut rect_txt, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        SelectObject(hdc_mem, old_f);
        let _ = DeleteObject(font);

        if !enabled && (key_type == "Num" || key_type == "Scroll") {
            draw_line(pixels, w, h, 52.0, 44.0, 98.0, 20.0, 2.8, 226, 75, 75);
        }

        // Right Tag: [ ON ] / [ OFF ]
        let font_tag = CreateFontW(14, 0, 0, 0, FW_BOLD.0 as i32, 0, 0, 0, 0, 0, 0, 0, 0, w!("Segoe UI"));
        let old_f2 = SelectObject(hdc_mem, font_tag);
        let tag_str = if enabled { "ON" } else { "OFF" };
        let tag_color = if enabled { COLORREF(0x00F8BD38) } else { COLORREF(0x0094A3B8) };
        SetTextColor(hdc_mem, tag_color);

        let mut rect_tag = RECT { left: 108, top: 16, right: w_px - 14, bottom: h_px - 16 };
        let wide_tag: Vec<u16> = tag_str.encode_utf16().collect();
        DrawTextW(hdc_mem, &mut wide_tag.clone(), &mut rect_tag, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        SelectObject(hdc_mem, old_f2);
        let _ = DeleteObject(font_tag);

        fix_gdi_text_alpha(pixels, w, h, rad);
        commit_layered_window(hwnd, x, y, w_px, h_px, hdc_mem, old_bmp, hbmp, hdc_screen);
    }
}

// =========================================================================
// THEME 3: NeumorphicKey — Tactile 3D physical keycap
// =========================================================================
fn render_neumorphic_key(
    hwnd: HWND,
    x: i32,
    y: i32,
    w_px: i32,
    h_px: i32,
    key_type: &str,
    enabled: bool,
) {
    let w = w_px as usize;
    let h = h_px as usize;

    unsafe {
        let hdc_screen = windows::Win32::Graphics::Gdi::GetDC(None);
        let hdc_mem = CreateCompatibleDC(hdc_screen);
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w_px,
                biHeight: -h_px,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut p_bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let hbmp = CreateDIBSection(hdc_mem, &bmi, DIB_RGB_COLORS, &mut p_bits, None, 0).unwrap_or_default();
        if hbmp.0.is_null() || p_bits.is_null() {
            let _ = DeleteDC(hdc_mem);
            windows::Win32::Graphics::Gdi::ReleaseDC(None, hdc_screen);
            return;
        }
        let old_bmp = SelectObject(hdc_mem, hbmp);
        let pixels = std::slice::from_raw_parts_mut(p_bits as *mut u8, w * h * 4);

        // Base plate (Dark charcoal #121418, alpha 252)
        let rad = (h_px as f32) * 0.16;
        draw_base_rounded_background(pixels, w, h, rad, 252.0, 16, 18, 22);

        // 3D Keycap bevel (size ~96x96)
        let km = 18.0f32;
        let kr = 14.0f32;
        // Keycap body fill
        draw_solid_round_rect(pixels, w, h, km, km, (w_px as f32) - km, (h_px as f32) - km, kr, 32, 36, 46, 255);
        // Highlight top rim
        draw_round_rect_stroke(pixels, w, h, km, km, (w_px as f32) - km, (h_px as f32) - km, kr, 1.6, 68, 76, 94);

        // LED Indicator on keycap
        let led_x = km + 14.0;
        let led_y = km + 14.0;
        let (lr, lg, lb) = if enabled { (56u8, 189u8, 248u8) } else { (64u8, 72u8, 88u8) };
        draw_solid_circle(pixels, w, h, led_x, led_y, 4.2, lr, lg, lb);

        // Center Text
        let txt = match key_type {
            "Caps" => if enabled { "ABC" } else { "abc" },
            "Num" => "123",
            _ => if enabled { "SCR" } else { "scr" },
        };

        let font = CreateFontW(30, 0, 0, 0, FW_BOLD.0 as i32, 0, 0, 0, 0, 0, 0, 0, 0, w!("Segoe UI"));
        let old_f = SelectObject(hdc_mem, font);
        let _ = SetBkMode(hdc_mem, TRANSPARENT);
        SetTextColor(hdc_mem, COLORREF(0x00FFFFFF));

        let mut rect_txt = RECT { left: km as i32, top: (km + 10.0) as i32, right: (w_px as f32 - km) as i32, bottom: (h_px as f32 - km) as i32 };
        let wide: Vec<u16> = txt.encode_utf16().collect();
        DrawTextW(hdc_mem, &mut wide.clone(), &mut rect_txt, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        SelectObject(hdc_mem, old_f);
        let _ = DeleteObject(font);

        if !enabled && (key_type == "Num" || key_type == "Scroll") {
            draw_line(pixels, w, h, km + 12.0, (h_px as f32) - km - 14.0, (w_px as f32) - km - 12.0, km + 14.0, 3.8, 226, 75, 75);
        }

        fix_gdi_text_alpha(pixels, w, h, rad);
        commit_layered_window(hwnd, x, y, w_px, h_px, hdc_mem, old_bmp, hbmp, hdc_screen);
    }
}

// =========================================================================
// THEME 4: DynamicIsland — Fluid rounded capsule
// =========================================================================
fn render_dynamic_island(
    hwnd: HWND,
    x: i32,
    y: i32,
    w_px: i32,
    h_px: i32,
    key_type: &str,
    enabled: bool,
) {
    let w = w_px as usize;
    let h = h_px as usize;

    unsafe {
        let hdc_screen = windows::Win32::Graphics::Gdi::GetDC(None);
        let hdc_mem = CreateCompatibleDC(hdc_screen);
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w_px,
                biHeight: -h_px,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut p_bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let hbmp = CreateDIBSection(hdc_mem, &bmi, DIB_RGB_COLORS, &mut p_bits, None, 0).unwrap_or_default();
        if hbmp.0.is_null() || p_bits.is_null() {
            let _ = DeleteDC(hdc_mem);
            windows::Win32::Graphics::Gdi::ReleaseDC(None, hdc_screen);
            return;
        }
        let old_bmp = SelectObject(hdc_mem, hbmp);
        let pixels = std::slice::from_raw_parts_mut(p_bits as *mut u8, w * h * 4);

        // Piano black capsule (radius = height/2 = 34px)
        let rad = (h_px as f32) / 2.0;
        draw_base_rounded_background(pixels, w, h, rad, 252.0, 8, 9, 12);
        draw_round_rect_stroke(pixels, w, h, 1.5, 1.5, (w_px as f32) - 1.5, (h_px as f32) - 1.5, rad - 1.0, 1.2, 45, 52, 66);

        // Left Jewel Dot
        let dot_x = 26.0f32;
        let dot_y = (h_px as f32) * 0.5;
        let (dr, dg, db) = if enabled { (34u8, 197u8, 94u8) } else { (100u8, 116u8, 139u8) };
        draw_solid_circle(pixels, w, h, dot_x, dot_y, 5.0, dr, dg, db);

        // Center text
        let txt = match key_type {
            "Caps" => if enabled { "ABC" } else { "abc" },
            "Num" => "123",
            _ => if enabled { "SCR" } else { "scr" },
        };

        let font = CreateFontW(23, 0, 0, 0, FW_BOLD.0 as i32, 0, 0, 0, 0, 0, 0, 0, 0, w!("Segoe UI"));
        let old_f = SelectObject(hdc_mem, font);
        let _ = SetBkMode(hdc_mem, TRANSPARENT);
        SetTextColor(hdc_mem, COLORREF(0x00FFFFFF));

        let mut rect_txt = RECT { left: 44, top: 8, right: 104, bottom: h_px - 8 };
        let wide: Vec<u16> = txt.encode_utf16().collect();
        DrawTextW(hdc_mem, &mut wide.clone(), &mut rect_txt, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        SelectObject(hdc_mem, old_f);
        let _ = DeleteObject(font);

        if !enabled && (key_type == "Num" || key_type == "Scroll") {
            draw_line(pixels, w, h, 48.0, 48.0, 100.0, 20.0, 2.8, 226, 75, 75);
        }

        // Right Status Pill
        let font_pill = CreateFontW(14, 0, 0, 0, FW_BOLD.0 as i32, 0, 0, 0, 0, 0, 0, 0, 0, w!("Segoe UI"));
        let old_f2 = SelectObject(hdc_mem, font_pill);
        let status_str = if enabled { "ON" } else { "OFF" };
        let status_color = if enabled { COLORREF(0x005EDB22) } else { COLORREF(0x008B9AA9) };
        SetTextColor(hdc_mem, status_color);

        let mut rect_pill = RECT { left: 106, top: 12, right: w_px - 14, bottom: h_px - 12 };
        let wide_s: Vec<u16> = status_str.encode_utf16().collect();
        DrawTextW(hdc_mem, &mut wide_s.clone(), &mut rect_pill, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        SelectObject(hdc_mem, old_f2);
        let _ = DeleteObject(font_pill);

        fix_gdi_text_alpha(pixels, w, h, rad);
        commit_layered_window(hwnd, x, y, w_px, h_px, hdc_mem, old_bmp, hbmp, hdc_screen);
    }
}

// =========================================================================
// Drawing & Layering primitives
// =========================================================================

fn draw_base_rounded_background(
    pixels: &mut [u8],
    w: usize,
    h: usize,
    radius: f32,
    alpha: f32,
    r_val: u8,
    g_val: u8,
    b_val: u8,
) {
    let cx = (w as f32) / 2.0;
    let cy = (h as f32) / 2.0;
    let hw = cx;
    let hh = cy;
    let safe_r = radius.min(hw).min(hh);

    for py in 0..h {
        for px in 0..w {
            let fx = px as f32 + 0.5;
            let fy = py as f32 + 0.5;

            let qx = (fx - cx).abs() - (hw - safe_r);
            let qy = (fy - cy).abs() - (hh - safe_r);
            let ext_x = qx.max(0.0);
            let ext_y = qy.max(0.0);
            let dist = (ext_x * ext_x + ext_y * ext_y).sqrt() + qx.max(qy).min(0.0) - safe_r;

            let factor = if dist <= -0.5 {
                1.0
            } else if dist < 0.5 {
                (0.5 - dist).clamp(0.0, 1.0)
            } else {
                0.0
            };

            if factor > 0.0 {
                let a = (alpha * factor) as u8;
                let r = ((r_val as f32 * (a as f32 / 255.0)) as u8).min(255);
                let g = ((g_val as f32 * (a as f32 / 255.0)) as u8).min(255);
                let b = ((b_val as f32 * (a as f32 / 255.0)) as u8).min(255);

                let idx = (py * w + px) * 4;
                pixels[idx] = b;
                pixels[idx + 1] = g;
                pixels[idx + 2] = r;
                pixels[idx + 3] = a;
            }
        }
    }
}

fn draw_round_rect_stroke(
    pixels: &mut [u8],
    w: usize,
    h: usize,
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
    radius: f32,
    stroke_w: f32,
    r: u8,
    g: u8,
    b: u8,
) {
    let half_s = stroke_w / 2.0;
    let min_x = (left - stroke_w - 1.0).max(0.0) as usize;
    let max_x = (right + stroke_w + 1.0).min(w as f32 - 1.0) as usize;
    let min_y = (top - stroke_w - 1.0).max(0.0) as usize;
    let max_y = (bottom + stroke_w + 1.0).min(h as f32 - 1.0) as usize;

    let cx = (left + right) / 2.0;
    let cy = (top + bottom) / 2.0;
    let hw = (right - left) / 2.0;
    let hh = (bottom - top) / 2.0;
    let safe_r = radius.min(hw).min(hh);

    for py in min_y..=max_y {
        for px in min_x..=max_x {
            let fx = px as f32 + 0.5;
            let fy = py as f32 + 0.5;

            let qx = (fx - cx).abs() - (hw - safe_r);
            let qy = (fy - cy).abs() - (hh - safe_r);
            let ext_x = qx.max(0.0);
            let ext_y = qy.max(0.0);
            let dist = (ext_x * ext_x + ext_y * ext_y).sqrt() + qx.max(qy).min(0.0) - safe_r;

            let edge_dist = dist.abs();
            if edge_dist <= half_s + 0.8 {
                let alpha_val = if edge_dist <= half_s - 0.3 {
                    1.0
                } else {
                    ((half_s + 0.8 - edge_dist) / 1.1).clamp(0.0, 1.0)
                };
                blend_pixel(pixels, w, h, px, py, r, g, b, alpha_val);
            }
        }
    }
}

fn draw_solid_round_rect(
    pixels: &mut [u8],
    w: usize,
    h: usize,
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
    radius: f32,
    r: u8,
    g: u8,
    b: u8,
    alpha: u8,
) {
    let min_x = left.max(0.0) as usize;
    let max_x = right.min(w as f32 - 1.0) as usize;
    let min_y = top.max(0.0) as usize;
    let max_y = bottom.min(h as f32 - 1.0) as usize;

    let cx = (left + right) / 2.0;
    let cy = (top + bottom) / 2.0;
    let hw = (right - left) / 2.0;
    let hh = (bottom - top) / 2.0;
    let safe_r = radius.min(hw).min(hh);
    let base_a = alpha as f32 / 255.0;

    for py in min_y..=max_y {
        for px in min_x..=max_x {
            let fx = px as f32 + 0.5;
            let fy = py as f32 + 0.5;

            let qx = (fx - cx).abs() - (hw - safe_r);
            let qy = (fy - cy).abs() - (hh - safe_r);
            let ext_x = qx.max(0.0);
            let ext_y = qy.max(0.0);
            let dist = (ext_x * ext_x + ext_y * ext_y).sqrt() + qx.max(qy).min(0.0) - safe_r;

            if dist <= 0.5 {
                let factor = if dist <= -0.5 {
                    1.0
                } else {
                    (0.5 - dist).clamp(0.0, 1.0)
                };
                blend_pixel(pixels, w, h, px, py, r, g, b, base_a * factor);
            }
        }
    }
}

fn draw_line(
    pixels: &mut [u8],
    w: usize,
    h: usize,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    stroke_w: f32,
    r: u8,
    g: u8,
    b: u8,
) {
    let half_s = stroke_w / 2.0;
    let min_x = (x1.min(x2) - stroke_w).max(0.0) as usize;
    let max_x = (x1.max(x2) + stroke_w).min(w as f32 - 1.0) as usize;
    let min_y = (y1.min(y2) - stroke_w).max(0.0) as usize;
    let max_y = (y1.max(y2) + stroke_w).min(h as f32 - 1.0) as usize;

    let line_dx = x2 - x1;
    let line_dy = y2 - y1;
    let line_len_sq = line_dx * line_dx + line_dy * line_dy;

    for py in min_y..=max_y {
        for px in min_x..=max_x {
            let fx = px as f32;
            let fy = py as f32;

            let t = if line_len_sq > 0.0 {
                (((fx - x1) * line_dx + (fy - y1) * line_dy) / line_len_sq).clamp(0.0, 1.0)
            } else {
                0.0
            };

            let proj_x = x1 + t * line_dx;
            let proj_y = y1 + t * line_dy;
            let dist = ((fx - proj_x) * (fx - proj_x) + (fy - proj_y) * (fy - proj_y)).sqrt();

            if dist <= half_s + 0.8 {
                let alpha_val = if dist <= half_s - 0.4 {
                    1.0
                } else {
                    ((half_s + 0.8 - dist) / 1.2).clamp(0.0, 1.0)
                };
                blend_pixel(pixels, w, h, px, py, r, g, b, alpha_val);
            }
        }
    }
}

fn draw_solid_circle(
    pixels: &mut [u8],
    w: usize,
    h: usize,
    cx: f32,
    cy: f32,
    radius: f32,
    r: u8,
    g: u8,
    b: u8,
) {
    let min_x = (cx - radius - 1.0).max(0.0) as usize;
    let max_x = (cx + radius + 1.0).min(w as f32 - 1.0) as usize;
    let min_y = (cy - radius - 1.0).max(0.0) as usize;
    let max_y = (cy + radius + 1.0).min(h as f32 - 1.0) as usize;

    for py in min_y..=max_y {
        for px in min_x..=max_x {
            let fx = px as f32;
            let fy = py as f32;
            let dist = ((fx - cx) * (fx - cx) + (fy - cy) * (fy - cy)).sqrt();

            if dist <= radius + 0.8 {
                let alpha_val = if dist <= radius - 0.4 {
                    1.0
                } else {
                    ((radius + 0.8 - dist) / 1.2).clamp(0.0, 1.0)
                };
                blend_pixel(pixels, w, h, px, py, r, g, b, alpha_val);
            }
        }
    }
}

#[inline]
fn blend_pixel(pixels: &mut [u8], w: usize, _h: usize, x: usize, y: usize, r: u8, g: u8, b: u8, alpha: f32) {
    let idx = (y * w + x) * 4;
    let a_byte = (255.0 * alpha) as u8;
    let premul_r = ((r as f32 * alpha) as u8).min(255);
    let premul_g = ((g as f32 * alpha) as u8).min(255);
    let premul_b = ((b as f32 * alpha) as u8).min(255);

    let cur_a = pixels[idx + 3] as f32 / 255.0;
    let new_a = alpha + cur_a * (1.0 - alpha);

    if new_a > 0.0 {
        pixels[idx] = (premul_b as f32 + pixels[idx] as f32 * (1.0 - alpha)).min(255.0) as u8;
        pixels[idx + 1] = (premul_g as f32 + pixels[idx + 1] as f32 * (1.0 - alpha)).min(255.0) as u8;
        pixels[idx + 2] = (premul_r as f32 + pixels[idx + 2] as f32 * (1.0 - alpha)).min(255.0) as u8;
        pixels[idx + 3] = (new_a * 255.0).max(a_byte as f32).min(255.0) as u8;
    }
}

fn fix_gdi_text_alpha(pixels: &mut [u8], w: usize, h: usize, bg_radius: f32) {
    let cx = (w as f32) / 2.0;
    let cy = (h as f32) / 2.0;
    let hw = cx;
    let hh = cy;
    let safe_r = bg_radius.min(hw).min(hh);

    for py in 0..h {
        for px in 0..w {
            let fx = px as f32 + 0.5;
            let fy = py as f32 + 0.5;

            let qx = (fx - cx).abs() - (hw - safe_r);
            let qy = (fy - cy).abs() - (hh - safe_r);
            let ext_x = qx.max(0.0);
            let ext_y = qy.max(0.0);
            let dist = (ext_x * ext_x + ext_y * ext_y).sqrt() + qx.max(qy).min(0.0) - safe_r;

            let idx = (py * w + px) * 4;
            if dist <= -0.5 {
                pixels[idx + 3] = 252;
            } else if dist < 0.5 {
                let factor = (0.5 - dist).clamp(0.0, 1.0);
                pixels[idx + 3] = ((252.0 * factor) as u8).max(pixels[idx + 3]);
            }
        }
    }
}

unsafe fn commit_layered_window(
    hwnd: HWND,
    x: i32,
    y: i32,
    w_px: i32,
    h_px: i32,
    hdc_mem: windows::Win32::Graphics::Gdi::HDC,
    old_bmp: windows::Win32::Graphics::Gdi::HGDIOBJ,
    hbmp: windows::Win32::Graphics::Gdi::HBITMAP,
    hdc_screen: windows::Win32::Graphics::Gdi::HDC,
) {
    let pt_dst = POINT { x, y };
    let pt_src = POINT { x: 0, y: 0 };
    let sz = SIZE { cx: w_px, cy: h_px };
    let blend = BLENDFUNCTION {
        BlendOp: AC_SRC_OVER as u8,
        BlendFlags: 0,
        SourceConstantAlpha: 255,
        AlphaFormat: AC_SRC_ALPHA as u8,
    };

    let _ = UpdateLayeredWindow(
        hwnd, None, Some(&pt_dst), Some(&sz), hdc_mem, Some(&pt_src), COLORREF(0), Some(&blend), ULW_ALPHA,
    );

    SelectObject(hdc_mem, old_bmp);
    let _ = DeleteObject(hbmp);
    let _ = DeleteDC(hdc_mem);
    windows::Win32::Graphics::Gdi::ReleaseDC(None, hdc_screen);
}
