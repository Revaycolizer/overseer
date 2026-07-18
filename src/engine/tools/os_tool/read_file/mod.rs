#[cfg(windows)]
pub mod windows;
#[cfg(not(windows))]
pub mod unix;

#[cfg(windows)]
pub use windows::windows_read_file_json;

#[cfg(not(windows))]
pub use unix::windows_read_file_json;
