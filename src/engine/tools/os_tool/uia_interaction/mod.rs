#[cfg(windows)]
pub mod windows_script1;
#[cfg(windows)]
pub mod windows_script2;
#[cfg(windows)]
pub mod windows_script3;
#[cfg(windows)]
pub mod windows_script4;
#[cfg(windows)]
pub mod windows;
#[cfg(not(windows))]
pub mod unix;

#[cfg(windows)]
pub use windows::windows_uia_interaction_json;

#[cfg(not(windows))]
pub use unix::windows_uia_interaction_json;
