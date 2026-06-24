use std::sync::atomic::{AtomicUsize, Ordering};

static ACTIVE_DOWNLOADS: AtomicUsize = AtomicUsize::new(0);

/// Keeps the system awake while at least one download job is running (Windows).
pub struct WakeGuard;

impl WakeGuard {
    pub fn acquire() -> Self {
        let prev = ACTIVE_DOWNLOADS.fetch_add(1, Ordering::SeqCst);
        if prev == 0 {
            set_prevent_sleep(true);
        }
        WakeGuard
    }
}

impl Drop for WakeGuard {
    fn drop(&mut self) {
        let prev = ACTIVE_DOWNLOADS.fetch_sub(1, Ordering::SeqCst);
        if prev == 1 {
            set_prevent_sleep(false);
        }
    }
}

#[cfg(windows)]
fn set_prevent_sleep(prevent: bool) {
    const ES_CONTINUOUS: u32 = 0x8000_0000;
    const ES_SYSTEM_REQUIRED: u32 = 0x0000_0001;

    unsafe extern "system" {
        fn SetThreadExecutionState(es_flags: u32) -> u32;
    }

    unsafe {
        if prevent {
            SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED);
        } else {
            SetThreadExecutionState(ES_CONTINUOUS);
        }
    }
}

#[cfg(not(windows))]
fn set_prevent_sleep(_prevent: bool) {}
