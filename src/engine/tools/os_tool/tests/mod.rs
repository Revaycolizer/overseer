pub mod inference;
// Live desktop-session smoke tests (screenshots, window/browser state, input).
// Gated so the default `cargo test` never touches real hardware or a display.
#[cfg(feature = "live-hardware-tests")]
pub mod live;
