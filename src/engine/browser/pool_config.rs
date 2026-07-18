pub struct PoolConfig {
    pub max_sessions: usize,
    pub stealth: bool,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self { max_sessions: 4, stealth: false }
    }
}
