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
        "CyberMinimal" => (168, 64),
        "DynamicIsland" => (156, 68),
        _ => (130, 130), // Default & Neumorphic
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
            168,
            130,
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

        // 1. Dark Opaque Background (alpha = 252 so nothing underneath bleeds through!)
        let bg_radius = (h_px as f32) * 0.17;
        let bg_alpha = 252.0f32;

        draw_base_rounded_background(pixels, w, h, bg_radius, bg_alpha, 14, 17, 24);

        // 2. Subtle, elegant accent border (No harsh neon, soft teal/cyan in ON, slate in OFF)
        let (border_r, border_g, border_b) = if enabled {
            (45u8, 212u8, 191u8) // Soft Teal/Cyan #2DD4BF
        } else {
            (51u8, 65u8, 85u8) // Slate 700
        };
        draw_round_rect_stroke(pixels, w, h, 1.5, 1.5, (w_px as f32) - 1.5, (h_px as f32) - 1.5, bg_radius - 1.0, 1.4, border_r, border_g, border_b);

        // 3. Top Indicator Glyph
        let badge_y = (h_px as f32) * 0.20;
        let center_x = (w_px as f32) * 0.5;

        match key_type {
            "Caps" => {
                let chev_w = (w_px as f32) * 0.09;
                let chev_h = (h_px as f32) * 0.07;
                let (cr, cg, cb) = if enabled { (45u8, 212u8, 191u8) } else { (100u8, 116u8, 139u8) };
                draw_line(pixels, w, h, center_x - chev_w, badge_y + chev_h, center_x, badge_y, 3.2, cr, cg, cb);
                draw_line(pixels, w, h, center_x, badge_y, center_x + chev_w, badge_y + chev_h, 3.2, cr, cg, cb);
            }
            "Num" => {
                let (cr, cg, cb) = if enabled { (45u8, 212u8, 191u8) } else { (100u8, 116u8, 139u8) };
                draw_solid_circle(pixels, w, h, center_x, badge_y + 3.0, (w_px as f32) * 0.040, cr, cg, cb);
            }
            _ => {
                let (cr, cg, cb) = if enabled { (45u8, 212u8, 191u8) } else { (100u8, 116u8, 139u8) };
                let sc_len = (h_px as f32) * 0.045;
                draw_line(pixels, w, h, center_x, badge_y - sc_len + 3.0, center_x, badge_y + sc_len + 3.0, 2.8, cr, cg, cb);
            }
        }

        // 4. Center Text: "ABC" (ON) / "abc" (OFF) for Caps; "123" for Num; "SCR" for Scroll
        let (center_text, font_size) = match key_type {
            "Caps" => {
                if enabled {
                    ("ABC", ((h_px as f32) * 0.23) as i32)
                } else {
                    ("abc", ((h_px as f32) * 0.22) as i32)
                }
            }
            "Num" => ("123", ((h_px as f32) * 0.23) as i32),
            _ => {
                if enabled {
                    ("SCR", ((h_px as f32) * 0.20) as i32)
                } else {
                    ("scr", ((h_px as f32) * 0.20) as i32)
                }
            }
        };

        let font_title = CreateFontW(
            font_size, 0, 0, 0, if enabled { FW_BOLD.0 as i32 } else { FW_SEMIBOLD.0 as i32 },
            0, 0, 0, 0, 0, 0, 0, 0, w!("Segoe UI"),
        );
        let old_font = SelectObject(hdc_mem, font_title);
        let _ = SetBkMode(hdc_mem, TRANSPARENT);
        SetTextColor(hdc_mem, COLORREF(0x00FFFFFF));

        let mut rect_title = RECT {
            left: 6,
            top: ((h_px as f32) * 0.35) as i32,
            right: w_px - 6,
            bottom: ((h_px as f32) * 0.65) as i32,
        };
        let wide_title: Vec<u16> = center_text.encode_utf16().collect();
        DrawTextW(hdc_mem, &mut wide_title.clone(), &mut rect_title, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        SelectObject(hdc_mem, old_font);
        let _ = DeleteObject(font_title);

        // Diagonal slash through 123 if Num Lock is OFF (or Scroll Lock OFF)
        if !enabled && (key_type == "Num" || key_type == "Scroll") {
            let slash_x1 = (w_px as f32) * 0.22;
            let slash_y1 = (h_px as f32) * 0.60;
            let slash_x2 = (w_px as f32) * 0.78;
            let slash_y2 = (h_px as f32) * 0.40;
            draw_line(pixels, w, h, slash_x1, slash_y1, slash_x2, slash_y2, 3.4, 226, 75, 75); // Soft muted coral slash
        }

        // 5. Bottom Status Pill Badge ("[ ● ON ]" / "[ ○ OFF ]")
        let pill_w = (w_px as f32) * 0.54;
        let pill_h = (h_px as f32) * 0.19;
        let pill_x = ((w_px as f32) - pill_w) / 2.0;
        let pill_y = (h_px as f32) * 0.70;
        let pill_rad = pill_h / 2.0;

        let (pill_r, pill_g, pill_b) = if enabled {
            (45u8, 212u8, 191u8) // Soft Teal
        } else {
            (100u8, 116u8, 139u8) // Slate 500
        };

        draw_solid_round_rect(pixels, w, h, pill_x, pill_y, pill_x + pill_w, pill_y + pill_h, pill_rad, pill_r, pill_g, pill_b, if enabled { 36 } else { 24 });
        draw_round_rect_stroke(pixels, w, h, pill_x, pill_y, pill_x + pill_w, pill_y + pill_h, pill_rad, 1.2, pill_r, pill_g, pill_b);

        let status_str = if enabled { "● ON" } else { "○ OFF" };
        let font_sub = CreateFontW(
            ((h_px as f32) * 0.12) as i32, 0, 0, 0, FW_BOLD.0 as i32,
            0, 0, 0, 0, 0, 0, 0, 0, w!("Segoe UI"),
        );
        let old_font2 = SelectObject(hdc_mem, font_sub);
        SetTextColor(hdc_mem, COLORREF(0x00FFFFFF));

        let mut rect_pill = RECT {
            left: pill_x as i32,
            top: pill_y as i32,
            right: (pill_x + pill_w) as i32,
            bottom: (pill_y + pill_h) as i32,
        };
        let wide_sub: Vec<u16> = status_str.encode_utf16().collect();
        DrawTextW(hdc_mem, &mut wide_sub.clone(), &mut rect_pill, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        SelectObject(hdc_mem, old_font2);
        let _ = DeleteObject(font_sub);

        fix_gdi_text_alpha(pixels, w, h);

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

        fix_gdi_text_alpha(pixels, w, h);
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

        fix_gdi_text_alpha(pixels, w, h);
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

        fix_gdi_text_alpha(pixels, w, h);
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
    let w_f = w as f32;
    let h_f = h as f32;

    for py in 0..h {
        for px in 0..w {
            let fx = px as f32;
            let fy = py as f32;

            let dx = if fx < radius {
                radius - fx
            } else if fx > w_f - 1.0 - radius {
                fx - (w_f - 1.0 - radius)
            } else {
                0.0
            };

            let dy = if fy < radius {
                radius - fy
            } else if fy > h_f - 1.0 - radius {
                fy - (h_f - 1.0 - radius)
            } else {
                0.0
            };

            let dist = (dx * dx + dy * dy).sqrt();
            let factor = if dist <= radius - 1.0 {
                1.0
            } else if dist < radius + 0.5 {
                (radius + 0.5 - dist).clamp(0.0, 1.0)
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
    let min_x = (left - stroke_w).max(0.0) as usize;
    let max_x = (right + stroke_w).min(w as f32 - 1.0) as usize;
    let min_y = (top - stroke_w).max(0.0) as usize;
    let max_y = (bottom + stroke_w).min(h as f32 - 1.0) as usize;

    for py in min_y..=max_y {
        for px in min_x..=max_x {
            let fx = px as f32;
            let fy = py as f32;

            let dx = if fx < left + radius {
                left + radius - fx
            } else if fx > right - radius {
                fx - (right - radius)
            } else {
                0.0
            };

            let dy = if fy < top + radius {
                top + radius - fy
            } else if fy > bottom - radius {
                fy - (bottom - radius)
            } else {
                0.0
            };

            let dist = if dx > 0.0 && dy > 0.0 {
                (dx * dx + dy * dy).sqrt() - radius
            } else if dx > 0.0 {
                dx - radius
            } else if dy > 0.0 {
                dy - radius
            } else {
                -((left + radius - fx).abs().min((right - radius - fx).abs()).min((top + radius - fy).abs()).min((bottom - radius - fy).abs()))
            };

            let edge_dist = dist.abs();
            if edge_dist <= half_s + 0.8 {
                let alpha_val = if edge_dist <= half_s - 0.4 {
                    1.0
                } else {
                    ((half_s + 0.8 - edge_dist) / 1.2).clamp(0.0, 1.0)
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

    let base_a = alpha as f32 / 255.0;

    for py in min_y..=max_y {
        for px in min_x..=max_x {
            let fx = px as f32;
            let fy = py as f32;

            let dx = if fx < left + radius {
                left + radius - fx
            } else if fx > right - radius {
                fx - (right - radius)
            } else {
                0.0
            };

            let dy = if fy < top + radius {
                top + radius - fy
            } else if fy > bottom - radius {
                fy - (bottom - radius)
            } else {
                0.0
            };

            let dist = if dx > 0.0 && dy > 0.0 {
                (dx * dx + dy * dy).sqrt()
            } else {
                0.0
            };

            if dist <= radius {
                blend_pixel(pixels, w, h, px, py, r, g, b, base_a);
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

fn fix_gdi_text_alpha(pixels: &mut [u8], w: usize, h: usize) {
    for ty in 0..h {
        for tx in 0..w {
            let idx = (ty * w + tx) * 4;
            let b = pixels[idx];
            let g = pixels[idx + 1];
            let r = pixels[idx + 2];

            if r > 40 || g > 40 || b > 40 {
                let brightness = (r.max(g).max(b) as f32) / 255.0;
                let target_a = (255.0 * brightness) as u8;
                pixels[idx] = ((b as f32 * brightness) as u8).min(255);
                pixels[idx + 1] = ((g as f32 * brightness) as u8).min(255);
                pixels[idx + 2] = ((r as f32 * brightness) as u8).min(255);
                pixels[idx + 3] = target_a.max(pixels[idx + 3]);
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
