#[cfg(windows)]
pub fn hide_console() -> u32 {
    0x08000000 // CREATE_NO_WINDOW
}

#[cfg(not(windows))]
pub fn hide_console() -> u32 {
    0
}
