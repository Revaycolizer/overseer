use std::sync::atomic::{AtomicU16, Ordering};

const BASE_PORT: u16 = 9300;

static NEXT_PORT: AtomicU16 = AtomicU16::new(0);

pub fn allocate_port() -> u16 {
    let offset = NEXT_PORT.fetch_add(1, Ordering::Relaxed);
    BASE_PORT + offset
}

pub fn release_port(_port: u16) {}
