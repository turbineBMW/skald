//! Thin wrapper over GstPlay.
use gst::prelude::*;

pub struct Engine {
    play: gst_play::Play,
}

impl Engine {
    pub fn new() -> Self {
        let play = gst_play::Play::new(None::<gst_play::PlayVideoRenderer>);
        Self { play }
    }
    pub fn load(&self, path: &std::path::Path) {
        self.play.set_uri(Some(&format!("file://{}", path.display())));
    }
    pub fn play(&self) { self.play.play(); }
    pub fn is_playing(&self) -> bool {
        // GstPlay has no state getter; infer from the pipeline element.
        self.play.pipeline().current_state() == gst::State::Playing
    }
    pub fn pause(&self) { self.play.pause(); }
    pub fn stop(&self) { self.play.stop(); }
    pub fn seek_ms(&self, ms: u64) { self.play.seek(gst::ClockTime::from_mseconds(ms)); }
    pub fn position_ms(&self) -> Option<u64> { self.play.position().map(|t| t.mseconds()) }
    pub fn duration_ms(&self) -> Option<u64> { self.play.duration().map(|t| t.mseconds()) }
    pub fn set_rate(&self, rate: f64) { self.play.set_rate(rate); }
    pub fn skip_ms(&self, delta: i64) {
        if let Some(p) = self.position_ms() {
            self.seek_ms((p as i64 + delta).max(0) as u64);
        }
    }
    /// Message bus for state/EOS/error/position-updated signals (drive from the GLib main loop).
    pub fn message_bus(&self) -> gst::Bus { self.play.message_bus() }
}
