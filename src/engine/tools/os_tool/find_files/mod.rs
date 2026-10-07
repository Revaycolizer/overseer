#[cfg(not(windows))]
pub mod unix;
#[cfg(not(windows))]
pub mod unix_scoring;
#[cfg(not(windows))]
pub mod unix_search;
#[cfg(windows)]
pub mod windows;

#[cfg(windows)]
pub use windows::windows_find_files_json;

#[cfg(not(windows))]
pub use unix::windows_find_files_json;
