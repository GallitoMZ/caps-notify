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
    size_str: String,
}

static HUD_STATE: Mutex<HudDisplayState> = Mutex::new(HudDisplayState {
    key_type: String::new(),
    enabled: false,
    theme: String::new(),
    size_str: String::new(),
});

/// Displays the floating HUD overlay indicator with true per-pixel alpha DWM rendering
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
            st.size_str = cfg.overlay_size.clone();
        }
    }

    let hwnd = get_or_create_overlay();
    if hwnd.0.is_null() {
        return;
    }

    let size_dim = match cfg.overlay_size.as_str() {
        "Small" => 108,
        "Large" => 156,
        _ => 130, // Default: Medium
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
        "TopRight" => (screen_w - size_dim - margin_x, margin_y),
        "CenterLeft" => (margin_x, (screen_h - size_dim) / 2),
        "Center" => ((screen_w - size_dim) / 2, (screen_h - size_dim) / 2),
        "CenterRight" => (screen_w - size_dim - margin_x, (screen_h - size_dim) / 2),
        "BottomLeft" => (margin_x, screen_h - size_dim - margin_bottom),
        "BottomCenter" => ((screen_w - size_dim) / 2, screen_h - size_dim - margin_bottom),
        "BottomRight" => (screen_w - size_dim - margin_x, screen_h - size_dim - margin_bottom),
        _ => ((screen_w - size_dim) / 2, margin_y), // Default: TopCenter
    };

    // Render selected theme
    if cfg.overlay_theme == "LenovoClassic" {
        render_lenovo_classic(hwnd, x, y, size_dim, key_type, enabled);
    } else {
        // Default: CapsNotifyModern (Unique, high-tech glass card)
        render_caps_notify_modern(hwnd, x, y, size_dim, key_type, enabled);
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
            130,
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
// DESIGN 1: CapsNotifyModern — Unique, elegant high-tech HUD (DEFAULT)
// =========================================================================
fn render_caps_notify_modern(
    hwnd: HWND,
    x: i32,
    y: i32,
    size: i32,
    key_type: &str,
    enabled: bool,
) {
    let w = size as usize;
    let h = size as usize;

    unsafe {
        let hdc_screen = windows::Win32::Graphics::Gdi::GetDC(None);
        let hdc_mem = CreateCompatibleDC(hdc_screen);

        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: size,
                biHeight: -size,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut p_bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let hbmp = CreateDIBSection(
            hdc_mem,
            &bmi,
            DIB_RGB_COLORS,
            &mut p_bits,
            None,
            0,
        ).unwrap_or_default();

        if hbmp.0.is_null() || p_bits.is_null() {
            let _ = DeleteDC(hdc_mem);
            windows::Win32::Graphics::Gdi::ReleaseDC(None, hdc_screen);
            return;
        }

        let old_bmp = SelectObject(hdc_mem, hbmp);
        let pixels = std::slice::from_raw_parts_mut(p_bits as *mut u8, w * h * 4);

        // 1. Dark Glass Card Background (Obsidian #0E121A, 93% opacity)
        let bg_radius = (size as f32) * 0.17;
        let bg_alpha = 236.0f32;

        for py in 0..h {
            for px in 0..w {
                let fx = px as f32;
                let fy = py as f32;

                let dx = if fx < bg_radius {
                    bg_radius - fx
                } else if fx > (size as f32) - 1.0 - bg_radius {
                    fx - ((size as f32) - 1.0 - bg_radius)
                } else {
                    0.0
                };

                let dy = if fy < bg_radius {
                    bg_radius - fy
                } else if fy > (size as f32) - 1.0 - bg_radius {
                    fy - ((size as f32) - 1.0 - bg_radius)
                } else {
                    0.0
                };

                let dist = (dx * dx + dy * dy).sqrt();
                let alpha_factor = if dist <= bg_radius - 1.0 {
                    1.0
                } else if dist < bg_radius + 0.5 {
                    (bg_radius + 0.5 - dist).clamp(0.0, 1.0)
                } else {
                    0.0
                };

                if alpha_factor > 0.0 {
                    let a = (bg_alpha * alpha_factor) as u8;
                    // Obsidian Slate #0E121A
                    let r = ((14.0 * (a as f32 / 255.0)) as u8).min(255);
                    let g = ((18.0 * (a as f32 / 255.0)) as u8).min(255);
                    let b = ((26.0 * (a as f32 / 255.0)) as u8).min(255);

                    let idx = (py * w + px) * 4;
                    pixels[idx] = b;
                    pixels[idx + 1] = g;
                    pixels[idx + 2] = r;
                    pixels[idx + 3] = a;
                }
            }
        }

        // 2. Luminous Accent Border
        let (border_r, border_g, border_b) = if enabled {
            (0u8, 245u8, 155u8) // Cyber Emerald
        } else {
            (71u8, 85u8, 105u8) // Slate 600
        };
        draw_round_rect_stroke(pixels, w, h, 2.0, 2.0, (size as f32) - 2.0, (size as f32) - 2.0, bg_radius - 1.0, 1.8, border_r, border_g, border_b);

        // 3. Top Illuminated Icon Badge
        let badge_y = (size as f32) * 0.22;
        let center_x = (size as f32) * 0.5;

        match key_type {
            "Caps" => {
                // Modern illuminated arrow chevron
                let chev_w = (size as f32) * 0.11;
                let chev_h = (size as f32) * 0.08;
                let (cr, cg, cb) = if enabled { (0u8, 245u8, 155u8) } else { (148u8, 163u8, 184u8) };
                draw_line(pixels, w, h, center_x - chev_w, badge_y + chev_h, center_x, badge_y, 3.8, cr, cg, cb);
                draw_line(pixels, w, h, center_x, badge_y, center_x + chev_w, badge_y + chev_h, 3.8, cr, cg, cb);
            }
            "Num" => {
                let (cr, cg, cb) = if enabled { (0u8, 245u8, 155u8) } else { (148u8, 163u8, 184u8) };
                draw_solid_circle(pixels, w, h, center_x, badge_y + 4.0, (size as f32) * 0.045, cr, cg, cb);
            }
            _ => {
                let (cr, cg, cb) = if enabled { (0u8, 245u8, 155u8) } else { (148u8, 163u8, 184u8) };
                let sc_len = (size as f32) * 0.05;
                draw_line(pixels, w, h, center_x, badge_y - sc_len + 4.0, center_x, badge_y + sc_len + 4.0, 3.2, cr, cg, cb);
            }
        }

        // 4. Center Key Name Title ("CAPS", "NUM", "SCROLL")
        let title_text = match key_type {
            "Caps" => "CAPS",
            "Num" => "NUM",
            _ => "SCROLL",
        };
        let font_title = CreateFontW(
            ((size as f32) * 0.20) as i32,
            0, 0, 0, FW_BOLD.0 as i32, 0, 0, 0, 0, 0, 0, 0, 0, w!("Segoe UI"),
        );
        let old_font = SelectObject(hdc_mem, font_title);
        let _ = SetBkMode(hdc_mem, TRANSPARENT);
        SetTextColor(hdc_mem, COLORREF(0x00FFFFFF));

        let mut rect_title = RECT {
            left: 8,
            top: ((size as f32) * 0.38) as i32,
            right: size - 8,
            bottom: ((size as f32) * 0.65) as i32,
        };
        let wide_title: Vec<u16> = title_text.encode_utf16().collect();
        DrawTextW(hdc_mem, &mut wide_title.clone(), &mut rect_title, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        SelectObject(hdc_mem, old_font);
        let _ = DeleteObject(font_title);

        // 5. Bottom Status Pill Badge ("[ ● ON ]" / "[ ○ OFF ]")
        let pill_w = (size as f32) * 0.56;
        let pill_h = (size as f32) * 0.20;
        let pill_x = ((size as f32) - pill_w) / 2.0;
        let pill_y = (size as f32) * 0.69;
        let pill_rad = pill_h / 2.0;

        let (pill_r, pill_g, pill_b) = if enabled {
            (0u8, 245u8, 155u8) // Emerald
        } else {
            (100u8, 116u8, 139u8) // Slate 500
        };

        // Fill pill
        draw_solid_round_rect(pixels, w, h, pill_x, pill_y, pill_x + pill_w, pill_y + pill_h, pill_rad, pill_r, pill_g, pill_b, if enabled { 45 } else { 30 });
        draw_round_rect_stroke(pixels, w, h, pill_x, pill_y, pill_x + pill_w, pill_y + pill_h, pill_rad, 1.4, pill_r, pill_g, pill_b);

        // Status text inside pill
        let status_str = if enabled { "● ON" } else { "○ OFF" };
        let font_sub = CreateFontW(
            ((size as f32) * 0.125) as i32,
            0, 0, 0, FW_BOLD.0 as i32, 0, 0, 0, 0, 0, 0, 0, 0, w!("Segoe UI"),
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

        // Make GDI text fully opaque in text areas
        fix_gdi_text_alpha(pixels, w, h, 0, size as usize, 0, size as usize);

        // UpdateLayeredWindow
        let pt_dst = POINT { x, y };
        let pt_src = POINT { x: 0, y: 0 };
        let sz = SIZE { cx: size, cy: size };
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
}

// =========================================================================
// DESIGN 2: LenovoClassic — Exact pixel-accurate reproduction of Lenovo OSD
// =========================================================================
fn render_lenovo_classic(
    hwnd: HWND,
    x: i32,
    y: i32,
    size: i32,
    key_type: &str,
    enabled: bool,
) {
    let w = size as usize;
    let h = size as usize;

    unsafe {
        let hdc_screen = windows::Win32::Graphics::Gdi::GetDC(None);
        let hdc_mem = CreateCompatibleDC(hdc_screen);

        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: size,
                biHeight: -size,
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

        // 1. Dark Rounded Background (#131315 at 87% opacity, 222 alpha)
        let bg_radius = (size as f32) * 0.15;
        let bg_alpha = 222.0f32;

        for py in 0..h {
            for px in 0..w {
                let fx = px as f32;
                let fy = py as f32;

                let dx = if fx < bg_radius {
                    bg_radius - fx
                } else if fx > (size as f32) - 1.0 - bg_radius {
                    fx - ((size as f32) - 1.0 - bg_radius)
                } else {
                    0.0
                };

                let dy = if fy < bg_radius {
                    bg_radius - fy
                } else if fy > (size as f32) - 1.0 - bg_radius {
                    fy - ((size as f32) - 1.0 - bg_radius)
                } else {
                    0.0
                };

                let dist = (dx * dx + dy * dy).sqrt();
                let alpha_factor = if dist <= bg_radius - 1.0 {
                    1.0
                } else if dist < bg_radius + 0.5 {
                    (bg_radius + 0.5 - dist).clamp(0.0, 1.0)
                } else {
                    0.0
                };

                if alpha_factor > 0.0 {
                    let a = (bg_alpha * alpha_factor) as u8;
                    // Pure charcoal #131315
                    let r = ((19.0 * (a as f32 / 255.0)) as u8).min(255);
                    let g = ((19.0 * (a as f32 / 255.0)) as u8).min(255);
                    let b = ((21.0 * (a as f32 / 255.0)) as u8).min(255);

                    let idx = (py * w + px) * 4;
                    pixels[idx] = b;
                    pixels[idx + 1] = g;
                    pixels[idx + 2] = r;
                    pixels[idx + 3] = a;
                }
            }
        }

        // Subtle 1px outer border
        draw_round_rect_stroke(pixels, w, h, 1.0, 1.0, (size as f32) - 1.0, (size as f32) - 1.0, bg_radius, 1.0, 48, 50, 56);

        // 2. Pure White Keycap Frame (#FFFFFF)
        // Margin ~17%, Corner radius 16px, thickness 5.5px
        let k_margin = (size as f32) * 0.17;
        let k_left = k_margin;
        let k_top = k_margin;
        let k_right = (size as f32) - k_margin;
        let k_bottom = (size as f32) - k_margin;
        let k_radius = (size as f32) * 0.14;
        let stroke_w = ((size as f32) * 0.046).max(5.0);

        draw_round_rect_stroke(pixels, w, h, k_left, k_top, k_right, k_bottom, k_radius, stroke_w, 255, 255, 255);

        // 3. Small Keycap Dish Arc nestled strictly inside bottom-right corner (never sticks out!)
        let dish_cx = k_right - (size as f32) * 0.12;
        let dish_cy = k_bottom - (size as f32) * 0.12;
        let dish_r = (size as f32) * 0.10;
        draw_arc(pixels, w, h, dish_cx, dish_cy, dish_r, 0.0, 90.0, stroke_w * 0.75, 255, 255, 255);

        // 4. Top-left Glyph
        match key_type {
            "Caps" => {
                // Clean Chevron ^
                let apex_x = k_left + (size as f32) * 0.12;
                let apex_y = k_top + (size as f32) * 0.09;
                let leg_w = (size as f32) * 0.06;
                let leg_h = (size as f32) * 0.06;
                draw_line(pixels, w, h, apex_x - leg_w, apex_y + leg_h, apex_x, apex_y, stroke_w * 0.85, 255, 255, 255);
                draw_line(pixels, w, h, apex_x, apex_y, apex_x + leg_w, apex_y + leg_h, stroke_w * 0.85, 255, 255, 255);
            }
            "Num" => {
                // Solid dot LED
                let dot_x = k_left + (size as f32) * 0.12;
                let dot_y = k_top + (size as f32) * 0.12;
                let dot_r = (size as f32) * 0.042;
                draw_solid_circle(pixels, w, h, dot_x, dot_y, dot_r, 255, 255, 255);
            }
            _ => {
                // Scroll arrow
                let sc_x = k_left + (size as f32) * 0.12;
                let sc_y = k_top + (size as f32) * 0.12;
                let sc_l = (size as f32) * 0.05;
                draw_line(pixels, w, h, sc_x, sc_y - sc_l, sc_x, sc_y + sc_l, stroke_w * 0.7, 255, 255, 255);
            }
        }

        // 5. Center Text (ABC / abc / 123)
        let (text, font_size, is_bold) = match key_type {
            "Caps" => {
                if enabled {
                    ("ABC", ((size as f32) * 0.24) as i32, true)
                } else {
                    ("abc", ((size as f32) * 0.23) as i32, false)
                }
            }
            "Num" => ("123", ((size as f32) * 0.23) as i32, true),
            _ => {
                if enabled {
                    ("SCR", ((size as f32) * 0.19) as i32, true)
                } else {
                    ("scr", ((size as f32) * 0.19) as i32, false)
                }
            }
        };

        let weight = if is_bold { FW_BOLD.0 as i32 } else { FW_SEMIBOLD.0 as i32 };
        let font_center = CreateFontW(
            font_size, 0, 0, 0, weight, 0, 0, 0, 0, 0, 0, 0, 0, w!("Segoe UI"),
        );
        let old_font = SelectObject(hdc_mem, font_center);
        let _ = SetBkMode(hdc_mem, TRANSPARENT);
        SetTextColor(hdc_mem, COLORREF(0x00FFFFFF));

        let y_adjust = ((size as f32) * 0.02) as i32;
        let mut rect_center = RECT {
            left: (k_left as i32) + 2,
            top: (k_top as i32) + y_adjust,
            right: (k_right as i32) - 2,
            bottom: (k_bottom as i32) + y_adjust,
        };
        let wide_str: Vec<u16> = text.encode_utf16().collect();
        DrawTextW(hdc_mem, &mut wide_str.clone(), &mut rect_center, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        SelectObject(hdc_mem, old_font);
        let _ = DeleteObject(font_center);

        // 6. Diagonal Slash if OFF for Num Lock or Scroll Lock
        if !enabled && (key_type == "Num" || key_type == "Scroll") {
            let slash_x1 = k_left - (size as f32) * 0.04;
            let slash_y1 = k_top - (size as f32) * 0.04;
            let slash_x2 = k_right + (size as f32) * 0.04;
            let slash_y2 = k_bottom + (size as f32) * 0.04;
            draw_line(pixels, w, h, slash_x1, slash_y1, slash_x2, slash_y2, stroke_w * 1.1, 255, 255, 255);
        }

        // Fix GDI text alpha
        fix_gdi_text_alpha(pixels, w, h, 0, size as usize, 0, size as usize);

        // UpdateLayeredWindow
        let pt_dst = POINT { x, y };
        let pt_src = POINT { x: 0, y: 0 };
        let sz = SIZE { cx: size, cy: size };
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
}

// =========================================================================
// Drawing helpers with anti-aliasing
// =========================================================================

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

fn draw_arc(
    pixels: &mut [u8],
    w: usize,
    h: usize,
    cx: f32,
    cy: f32,
    radius: f32,
    start_deg: f32,
    end_deg: f32,
    stroke_w: f32,
    r: u8,
    g: u8,
    b: u8,
) {
    let half_s = stroke_w / 2.0;
    let min_x = (cx - radius - stroke_w).max(0.0) as usize;
    let max_x = (cx + radius + stroke_w).min(w as f32 - 1.0) as usize;
    let min_y = (cy - radius - stroke_w).max(0.0) as usize;
    let max_y = (cy + radius + stroke_w).min(h as f32 - 1.0) as usize;

    let pi = std::f32::consts::PI;

    for py in min_y..=max_y {
        for px in min_x..=max_x {
            let fx = px as f32;
            let fy = py as f32;
            let dist = ((fx - cx) * (fx - cx) + (fy - cy) * (fy - cy)).sqrt();
            let rad_dist = (dist - radius).abs();

            if rad_dist <= half_s + 0.8 {
                let mut angle_deg = (fy - cy).atan2(fx - cx) * 180.0 / pi;
                if angle_deg < 0.0 {
                    angle_deg += 360.0;
                }

                if angle_deg >= start_deg && angle_deg <= end_deg {
                    let alpha_val = if rad_dist <= half_s - 0.4 {
                        1.0
                    } else {
                        ((half_s + 0.8 - rad_dist) / 1.2).clamp(0.0, 1.0)
                    };
                    blend_pixel(pixels, w, h, px, py, r, g, b, alpha_val);
                }
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

fn fix_gdi_text_alpha(pixels: &mut [u8], w: usize, _h: usize, min_x: usize, max_x: usize, min_y: usize, max_y: usize) {
    for ty in min_y..max_y {
        for tx in min_x..max_x {
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
