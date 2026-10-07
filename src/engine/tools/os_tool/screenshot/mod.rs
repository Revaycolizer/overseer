#[cfg(not(windows))]
pub mod linux;
#[cfg(not(windows))]
pub mod macos;
#[cfg(not(windows))]
pub mod unix;
#[cfg(windows)]
pub mod windows;

#[cfg(windows)]
pub use windows::capture_windows_screenshot_json;

#[cfg(not(windows))]
pub use unix::capture_windows_screenshot_json;
