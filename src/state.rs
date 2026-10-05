use std::sync::atomic::{AtomicBool, Ordering};
use once_cell::sync::Lazy;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, VK_CAPITAL, VK_NUMLOCK, VK_SCROLL};

pub static STATE: Lazy<LockState> = Lazy::new(LockState::new);

pub struct LockState {
    pub caps: AtomicBool,
    pub num: AtomicBool,
    pub scroll: AtomicBool,
}

impl LockState {
    pub fn new() -> Self {
        let state = Self {
            caps: AtomicBool::new(false),
            num: AtomicBool::new(false),
            scroll: AtomicBool::new(false),
        };
        state.read_all_from_system();
        state
    }

    /// Synchronizes internal atomic flags with Windows GetKeyState
    pub fn read_all_from_system(&self) {
        unsafe {
            // In Windows, the low-order bit (0x0001) indicates toggle state
            let caps_on = (GetKeyState(VK_CAPITAL.0 as i32) & 0x0001) != 0;
            let num_on = (GetKeyState(VK_NUMLOCK.0 as i32) & 0x0001) != 0;
            let scroll_on = (GetKeyState(VK_SCROLL.0 as i32) & 0x0001) != 0;

            self.caps.store(caps_on, Ordering::SeqCst);
            self.num.store(num_on, Ordering::SeqCst);
            self.scroll.store(scroll_on, Ordering::SeqCst);
        }
    }

    #[inline]
    pub fn is_caps_on(&self) -> bool {
        self.caps.load(Ordering::SeqCst)
    }

    #[inline]
    pub fn is_num_on(&self) -> bool {
        self.num.load(Ordering::SeqCst)
    }

    #[inline]
    pub fn is_scroll_on(&self) -> bool {
        self.scroll.load(Ordering::SeqCst)
    }
}
