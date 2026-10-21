//! Monotonic and wall-clock time values that work on every Incular target.
//!
//! On native targets these are exactly [`std::time`]'s types, so values
//! created through either path are interchangeable. On `wasm32` browser
//! targets, where `std::time::Instant::now` panics, they are backed by the
//! browser's `performance.now()` and `Date.now()` clocks.

pub use web_time::{Duration, Instant, SystemTime, SystemTimeError, UNIX_EPOCH};
