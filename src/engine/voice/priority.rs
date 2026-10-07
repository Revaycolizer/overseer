pub fn set_thread_priority_and_affinity() {
    #[cfg(target_os = "linux")]
    unsafe {
        // Set nice value to -10 (high priority)
        libc::setpriority(libc::PRIO_PROCESS, 0, -10);

        // Pin to CPU 0
        let mut cpuset: libc::cpu_set_t = std::mem::zeroed();
        let cpuset_ptr = &mut cpuset as *mut libc::cpu_set_t as *mut libc::c_ulong;
        *cpuset_ptr |= 1;
        libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &cpuset);
    }

    #[cfg(target_os = "windows")]
    unsafe {
        use windows::Win32::System::Threading::{
            GetCurrentThread, SetThreadAffinityMask, SetThreadPriority, THREAD_PRIORITY_HIGHEST,
        };
        let thread_handle = GetCurrentThread();
        let _ = SetThreadPriority(thread_handle, THREAD_PRIORITY_HIGHEST);
        let _ = SetThreadAffinityMask(thread_handle, 1);
    }

    #[cfg(target_os = "macos")]
    unsafe {
        libc::setpriority(libc::PRIO_PROCESS, 0, -10);
    }
}
