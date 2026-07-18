#[cfg(windows)]
pub mod windows;
#[cfg(not(windows))]
pub mod unix;

#[cfg(windows)]
pub use windows::windows_open_file_json;

#[cfg(not(windows))]
pub use unix::windows_open_file_json;
