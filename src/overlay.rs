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
        "Small" => 104,
        "Large" => 156,
        _ => 128, // Default: Medium (identical to Lenovo OSD)
    };

    let (screen_w, screen_h) = unsafe {
        (
            GetSystemMetrics(SM_CXSCREEN),
            GetSystemMetrics(SM_CYSCREEN),
        )
    };

    let margin_x = 40;
    let margin_y = 50;
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

    // Render bitmap directly to window surface via UpdateLayeredWindow
    render_hud(hwnd, x, y, size_dim, key_type, enabled, &cfg.overlay_theme);

    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        // Reset timer dynamically on every single key press
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
            128,
            128,
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

/// Renders the Lenovo OSD keycap badge onto a 32-bit ARGB DIB and calls UpdateLayeredWindow
fn render_hud(
    hwnd: HWND,
    x: i32,
    y: i32,
    size: i32,
    key_type: &str,
    enabled: bool,
    theme: &str,
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
                biHeight: -size, // Top-down DIB
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

        // 1. Draw rounded dark translucent background with anti-aliasing
        let bg_radius = (size as f32) * 0.16; // Rounded corner radius
        let bg_alpha = 224.0f32; // ~88% opacity

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
                    // Dark charcoal/slate background: #17181C
                    let r = ((23.0 * (a as f32 / 255.0)) as u8).min(255);
                    let g = ((24.0 * (a as f32 / 255.0)) as u8).min(255);
                    let b = ((28.0 * (a as f32 / 255.0)) as u8).min(255);

                    let idx = (py * w + px) * 4;
                    pixels[idx] = b;
                    pixels[idx + 1] = g;
                    pixels[idx + 2] = r;
                    pixels[idx + 3] = a;
                }
            }
        }

        // Color palette for keycap and text
        let is_accent_theme = theme == "AccentColor";
        let (fg_r, fg_g, fg_b) = if is_accent_theme && enabled {
            (46u8, 213u8, 115u8) // Emerald Green
        } else if !enabled && is_accent_theme {
            (160u8, 168u8, 185u8) // Muted Gray
        } else {
            (255u8, 255u8, 255u8) // Pure Lenovo OEM White
        };

        // 2. Draw Keycap Outline + Dish Arc
        let key_margin = (size as f32) * 0.14;
        let key_radius = (size as f32) * 0.125;
        let stroke_w = ((size as f32) * 0.038).max(3.5);

        let k_left = key_margin;
        let k_top = key_margin;
        let k_right = (size as f32) - key_margin;
        let k_bottom = (size as f32) - key_margin;

        // Draw rounded rectangle stroke
        draw_round_rect_stroke(pixels, w, h, k_left, k_top, k_right, k_bottom, key_radius, stroke_w, fg_r, fg_g, fg_b);

        // Keycap 3D dish arc at bottom-right (identical to Lenovo design in images)
        let dish_cx = k_right - (key_radius * 0.45);
        let dish_cy = k_bottom - (key_radius * 0.45);
        let dish_r = (size as f32) * 0.15;
        draw_arc(pixels, w, h, dish_cx, dish_cy, dish_r, 45.0, 150.0, stroke_w * 0.75, fg_r, fg_g, fg_b);

        // 3. Draw Top-Left Glyph
        match key_type {
            "Caps" => {
                // Chevron ^ (arrow up)
                let chev_cx = k_left + (size as f32) * 0.11;
                let chev_top = k_top + (size as f32) * 0.085;
                let chev_w = (size as f32) * 0.07;
                let chev_h = (size as f32) * 0.06;
                draw_line(pixels, w, h, chev_cx - chev_w, chev_top + chev_h, chev_cx, chev_top, stroke_w * 0.9, fg_r, fg_g, fg_b);
                draw_line(pixels, w, h, chev_cx, chev_top, chev_cx + chev_w, chev_top + chev_h, stroke_w * 0.9, fg_r, fg_g, fg_b);
            }
            "Num" => {
                // Solid circle LED dot
                let dot_cx = k_left + (size as f32) * 0.11;
                let dot_cy = k_top + (size as f32) * 0.11;
                let dot_r = (size as f32) * 0.04;
                draw_solid_circle(pixels, w, h, dot_cx, dot_cy, dot_r, fg_r, fg_g, fg_b);
            }
            _ => {
                // Scroll lock vertical arrows
                let sc_cx = k_left + (size as f32) * 0.11;
                let sc_cy = k_top + (size as f32) * 0.11;
                let sc_len = (size as f32) * 0.055;
                draw_line(pixels, w, h, sc_cx, sc_cy - sc_len, sc_cx, sc_cy + sc_len, stroke_w * 0.75, fg_r, fg_g, fg_b);
                draw_line(pixels, w, h, sc_cx - 3.0, sc_cy - sc_len + 3.0, sc_cx, sc_cy - sc_len, stroke_w * 0.75, fg_r, fg_g, fg_b);
                draw_line(pixels, w, h, sc_cx + 3.0, sc_cy - sc_len + 3.0, sc_cx, sc_cy - sc_len, stroke_w * 0.75, fg_r, fg_g, fg_b);
            }
        }

        // 4. Render Center Text with GDI Font
        let (text, font_size) = match key_type {
            "Caps" => {
                if enabled {
                    ("ABC", ((size as f32) * 0.22) as i32)
                } else {
                    ("abc", ((size as f32) * 0.21) as i32)
                }
            }
            "Num" => ("123", ((size as f32) * 0.22) as i32),
            _ => {
                if enabled {
                    ("SCR", ((size as f32) * 0.19) as i32)
                } else {
                    ("scr", ((size as f32) * 0.19) as i32)
                }
            }
        };

        let font_weight = if enabled { FW_BOLD.0 as i32 } else { FW_SEMIBOLD.0 as i32 };
        let hfont = CreateFontW(
            font_size,
            0,
            0,
            0,
            font_weight,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            w!("Segoe UI"),
        );
        let old_font = SelectObject(hdc_mem, hfont);
        let _ = SetBkMode(hdc_mem, TRANSPARENT);
        SetTextColor(hdc_mem, COLORREF(0x00FFFFFF));

        // Center vertical alignment with slight optical offset
        let y_offset = ((size as f32) * 0.02) as i32;
        let mut text_rect = RECT {
            left: (k_left as i32) + 2,
            top: (k_top as i32) + y_offset,
            right: (k_right as i32) - 2,
            bottom: (k_bottom as i32) + y_offset,
        };

        let wide_str: Vec<u16> = text.encode_utf16().collect();
        DrawTextW(
            hdc_mem,
            &mut wide_str.clone(),
            &mut text_rect,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );

        SelectObject(hdc_mem, old_font);
        let _ = DeleteObject(hfont);

        // Fix GDI text alpha & apply colors in text area
        let text_y_min = (text_rect.top.max(0)) as usize;
        let text_y_max = (text_rect.bottom.min(size)) as usize;
        let text_x_min = (text_rect.left.max(0)) as usize;
        let text_x_max = (text_rect.right.min(size)) as usize;

        for ty in text_y_min..text_y_max {
            for tx in text_x_min..text_x_max {
                let idx = (ty * w + tx) * 4;
                let b = pixels[idx];
                let g = pixels[idx + 1];
                let r = pixels[idx + 2];

                // If GDI rendered text here
                if r > 60 || g > 60 || b > 60 {
                    let brightness = (r.max(g).max(b) as f32) / 255.0;
                    let target_a = (255.0 * brightness) as u8;
                    let out_r = ((fg_r as f32 * brightness) as u8).min(255);
                    let out_g = ((fg_g as f32 * brightness) as u8).min(255);
                    let out_b = ((fg_b as f32 * brightness) as u8).min(255);

                    pixels[idx] = out_b;
                    pixels[idx + 1] = out_g;
                    pixels[idx + 2] = out_r;
                    pixels[idx + 3] = target_a;
                }
            }
        }

        // 5. Diagonal Slash if OFF for Num/Scroll (or Image 4 style)
        if !enabled && (key_type == "Num" || key_type == "Scroll") {
            let slash_x1 = k_left - (size as f32) * 0.05;
            let slash_y1 = k_top - (size as f32) * 0.05;
            let slash_x2 = k_right + (size as f32) * 0.05;
            let slash_y2 = k_bottom + (size as f32) * 0.05;
            let slash_w = stroke_w * 1.15;
            draw_line(pixels, w, h, slash_x1, slash_y1, slash_x2, slash_y2, slash_w, fg_r, fg_g, fg_b);
        }

        // 6. UpdateLayeredWindow atomic call to DWM
        let pt_dst = POINT { x, y };
        let pt_src = POINT { x: 0, y: 0 };
        let sz = SIZE {
            cx: size,
            cy: size,
        };

        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };

        let _ = UpdateLayeredWindow(
            hwnd,
            None,
            Some(&pt_dst),
            Some(&sz),
            hdc_mem,
            Some(&pt_src),
            COLORREF(0),
            Some(&blend),
            ULW_ALPHA,
        );

        SelectObject(hdc_mem, old_bmp);
        let _ = DeleteObject(hbmp);
        let _ = DeleteDC(hdc_mem);
        windows::Win32::Graphics::Gdi::ReleaseDC(None, hdc_screen);
    }
}

// Helper: Anti-aliased stroke for rounded rectangle
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

            // Distance to inner rounded box
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

// Helper: Anti-aliased line
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

// Helper: Anti-aliased circle
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

// Helper: Anti-aliased Arc
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
