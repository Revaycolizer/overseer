#[cfg(windows)]
pub mod windows;
#[cfg(not(windows))]
pub mod macos;
#[cfg(not(windows))]
pub mod linux;
#[cfg(not(windows))]
pub mod unix;

#[cfg(windows)]
pub use windows::windows_desktop_state_json;

#[cfg(not(windows))]
pub use unix::windows_desktop_state_json;
