use windows::core::w;
use windows::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE, REG_SZ,
};

const RUN_KEY_PATH: windows::core::PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const APP_KEY_NAME: windows::core::PCWSTR = w!("CapsNotify");

/// Checks if autostart entry is configured in HKCU Run key
pub fn is_enabled() -> bool {
    unsafe {
        let mut hkey = HKEY::default();
        if RegOpenKeyExW(HKEY_CURRENT_USER, RUN_KEY_PATH, 0, KEY_READ, &mut hkey).is_ok() {
            let res = RegQueryValueExW(hkey, APP_KEY_NAME, None, None, None, None);
            let _ = RegCloseKey(hkey);
            res.is_ok()
        } else {
            false
        }
    }
}

/// Adds current executable path with --minimized flag to HKCU Run key
pub fn enable() -> Result<(), Box<dyn std::error::Error>> {
    let current_exe = std::env::current_exe()?;
    let val_str = format!("\"{}\" --minimized", current_exe.to_string_lossy());
    let wide_val: Vec<u16> = val_str.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        let mut hkey = HKEY::default();
        RegOpenKeyExW(HKEY_CURRENT_USER, RUN_KEY_PATH, 0, KEY_SET_VALUE, &mut hkey).ok()?;

        let slice = std::slice::from_raw_parts(wide_val.as_ptr() as *const u8, wide_val.len() * 2);
        RegSetValueExW(hkey, APP_KEY_NAME, 0, REG_SZ, Some(slice)).ok()?;

        let _ = RegCloseKey(hkey);
    }
    Ok(())
}

/// Removes current executable from HKCU Run key
pub fn disable() -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        let mut hkey = HKEY::default();
        RegOpenKeyExW(HKEY_CURRENT_USER, RUN_KEY_PATH, 0, KEY_SET_VALUE, &mut hkey).ok()?;
        let _ = RegDeleteValueW(hkey, APP_KEY_NAME);
        let _ = RegCloseKey(hkey);
    }
    Ok(())
}
