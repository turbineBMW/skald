//! Whispersync push throttle: every `INTERVAL` while playing, plus on pause/seek/quit.
use std::cell::Cell;
use std::time::{Duration, Instant};

pub const INTERVAL: Duration = Duration::from_secs(30);

pub struct SyncState {
    last_pushed_ms: Cell<Option<u64>>,
    last_push_at: Cell<Instant>,
}

impl Default for SyncState {
    fn default() -> Self { Self { last_pushed_ms: Cell::new(None), last_push_at: Cell::new(Instant::now() - INTERVAL) } }
}

impl SyncState {
    /// Whether a push should happen now. `force` = pause/seek/quit.
    pub fn should_push(&self, position_ms: u64, force: bool) -> bool {
        if self.last_pushed_ms.get() == Some(position_ms) { return false; }
        force || self.last_push_at.get().elapsed() >= INTERVAL
    }
    pub fn mark(&self, position_ms: u64) {
        self.last_pushed_ms.set(Some(position_ms));
        self.last_push_at.set(Instant::now());
    }
    pub fn reset(&self) { let d = Self::default(); self.last_pushed_ms.set(d.last_pushed_ms.get()); self.last_push_at.set(d.last_push_at.get()); }
}
