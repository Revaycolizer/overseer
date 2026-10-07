#[cfg(target_os = "windows")]
pub fn get_idle_secs() -> u64 {
    use windows::Win32::System::SystemInformation::GetTickCount;
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};

    unsafe {
        let mut info = LASTINPUTINFO {
            cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
            dwTime: 0,
        };
        if GetLastInputInfo(&mut info).as_bool() {
            let idle_ms = GetTickCount().saturating_sub(info.dwTime);
            (idle_ms / 1000) as u64
        } else {
            0
        }
    }
}
