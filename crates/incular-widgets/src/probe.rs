//! TEMPORARY timing probe. Not for commit.
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
pub static NANOS: [AtomicU64; 16] = [const { AtomicU64::new(0) }; 16];
pub static CALLS: [AtomicU64; 16] = [const { AtomicU64::new(0) }; 16];
pub struct Span(usize, Instant);
pub fn span(slot: usize) -> Span {
    Span(slot, Instant::now())
}
impl Drop for Span {
    fn drop(&mut self) {
        NANOS[self.0].fetch_add(self.1.elapsed().as_nanos() as u64, Ordering::Relaxed);
        CALLS[self.0].fetch_add(1, Ordering::Relaxed);
    }
}
pub fn report(frames: u64) {
    for slot in 0..16 {
        let calls = CALLS[slot].swap(0, Ordering::Relaxed);
        let nanos = NANOS[slot].swap(0, Ordering::Relaxed);
        if calls > 0 {
            println!("  slot {slot}: {:.2} us/frame, {:.2} calls/frame, {} ns/call", nanos as f64 / frames as f64 / 1000., calls as f64 / frames as f64, nanos / calls);
        }
    }
}
