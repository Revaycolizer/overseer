#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum PressureLevel {
    Ok,
    Warning,
    Critical,
    Blocked,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct MemoryReport {
    pub total_ram_mb: u64,
    pub used_ram_mb: u64,
    pub available_ram_mb: u64,
    pub browser_pool_mb: u64,
    pub active_sessions: usize,
    pub pressure_level: PressureLevel,
    pub can_create_session: bool,
}

pub fn check_pressure(
    total_ram_mb: u64,
    used_ram_mb: u64,
    pool_mb: u64,
    active_sessions: usize,
    per_session_mb: u64,
) -> MemoryReport {
    let pct = if total_ram_mb > 0 { (used_ram_mb as f64 / total_ram_mb as f64) * 100.0 } else { 0.0 };
    let available = total_ram_mb.saturating_sub(used_ram_mb);
    let next_would_use = pool_mb + per_session_mb;

    let (pressure_level, can_create_session) = if pct >= 95.0 || next_would_use >= available {
        (PressureLevel::Blocked, false)
    } else if pct >= 85.0 {
        (PressureLevel::Critical, next_would_use < available)
    } else if pct >= 70.0 {
        (PressureLevel::Warning, true)
    } else {
        (PressureLevel::Ok, true)
    };

    MemoryReport {
        total_ram_mb,
        used_ram_mb,
        available_ram_mb: available,
        browser_pool_mb: pool_mb,
        active_sessions,
        pressure_level,
        can_create_session,
    }
}

pub fn current_report(pool_mb: u64, active_sessions: usize) -> MemoryReport {
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    let total = sys.total_memory() / 1024 / 1024;
    let used = sys.used_memory() / 1024 / 1024;
    check_pressure(total, used, pool_mb, active_sessions, 90)
}
