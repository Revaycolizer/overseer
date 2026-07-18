#[cfg(windows)]
pub mod windows_script1;
#[cfg(windows)]
pub mod windows_script2;
#[cfg(windows)]
pub mod windows;
#[cfg(not(windows))]
pub mod unix;

#[cfg(windows)]
pub use windows::windows_accessibility_state_json;

#[cfg(not(windows))]
pub use unix::windows_accessibility_state_json;
