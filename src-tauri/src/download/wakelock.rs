use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

static ACTIVE_DOWNLOADS: AtomicUsize = AtomicUsize::new(0);
static KEEPALIVE_RUNNING: AtomicBool = AtomicBool::new(false);

/// Keeps the system awake and reduces Windows background throttling while downloads run.
pub struct WakeGuard;

impl WakeGuard {
    pub fn acquire() -> Self {
        let prev = ACTIVE_DOWNLOADS.fetch_add(1, Ordering::SeqCst);
        if prev == 0 {
            set_background_resistant(true);
        }
        WakeGuard
    }
}

impl Drop for WakeGuard {
    fn drop(&mut self) {
        let prev = ACTIVE_DOWNLOADS.fetch_sub(1, Ordering::SeqCst);
        if prev == 1 {
            set_background_resistant(false);
        }
    }
}

#[cfg(windows)]
fn set_background_resistant(enable: bool) {
    if enable {
        set_execution_state(true);
        set_power_throttling(false);
        start_keepalive();
    } else if ACTIVE_DOWNLOADS.load(Ordering::SeqCst) == 0 {
        set_execution_state(false);
        set_power_throttling(true);
    }
}

#[cfg(windows)]
fn start_keepalive() {
    if KEEPALIVE_RUNNING
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }

    thread::spawn(|| {
        loop {
            thread::sleep(Duration::from_secs(45));
            if ACTIVE_DOWNLOADS.load(Ordering::SeqCst) == 0 {
                KEEPALIVE_RUNNING.store(false, Ordering::SeqCst);
                break;
            }
            set_execution_state(true);
        }
    });
}

#[cfg(not(windows))]
fn set_background_resistant(enable: bool) {
    let _ = enable;
}

#[cfg(windows)]
fn set_execution_state(enable: bool) {
    const ES_CONTINUOUS: u32 = 0x8000_0000;
    const ES_SYSTEM_REQUIRED: u32 = 0x0000_0001;
    const ES_AWAYMODE_REQUIRED: u32 = 0x0000_0040;

    unsafe extern "system" {
        fn SetThreadExecutionState(es_flags: u32) -> u32;
    }

    unsafe {
        if enable {
            SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_AWAYMODE_REQUIRED);
        } else {
            SetThreadExecutionState(ES_CONTINUOUS);
        }
    }
}

#[cfg(windows)]
fn set_power_throttling(throttle: bool) {
    #[repr(C)]
    struct ProcessPowerThrottlingState {
        version: u32,
        control_mask: u32,
        state_mask: u32,
    }

    const PROCESS_POWER_THROTTLING_CURRENT_VERSION: u32 = 1;
    const PROCESS_POWER_THROTTLING_EXECUTION_SPEED: u32 = 0x1;
    const PROCESS_POWER_THROTTLING: i32 = 4;

    unsafe extern "system" {
        fn GetCurrentProcess() -> isize;
        fn SetProcessInformation(
            h_process: isize,
            information_class: i32,
            process_information: *mut std::ffi::c_void,
            process_information_size: u32,
        ) -> i32;
    }

    let mut state = ProcessPowerThrottlingState {
        version: PROCESS_POWER_THROTTLING_CURRENT_VERSION,
        control_mask: PROCESS_POWER_THROTTLING_EXECUTION_SPEED,
        state_mask: if throttle {
            PROCESS_POWER_THROTTLING_EXECUTION_SPEED
        } else {
            0
        },
    };

    unsafe {
        let _ = SetProcessInformation(
            GetCurrentProcess(),
            PROCESS_POWER_THROTTLING,
            (&mut state as *mut ProcessPowerThrottlingState).cast(),
            std::mem::size_of::<ProcessPowerThrottlingState>() as u32,
        );
    }
}
