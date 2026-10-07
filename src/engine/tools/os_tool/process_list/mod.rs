#[cfg(not(windows))]
pub mod unix;
#[cfg(windows)]
pub mod windows;

#[cfg(windows)]
pub use windows::windows_process_list_json;

#[cfg(not(windows))]
pub use unix::windows_process_list_json;
