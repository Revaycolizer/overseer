mod linux;
mod macos;
mod windows;

pub fn get_system_idle_secs() -> u64 {
    #[cfg(target_os = "windows")]
    {
        windows::get_idle_secs()
    }

    #[cfg(target_os = "macos")]
    {
        macos::get_idle_secs()
    }

    #[cfg(target_os = "linux")]
    {
        linux::get_idle_secs()
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        0
    }
}
