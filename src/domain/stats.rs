use super::events::StreamEvent;

// ---------------------------------------------------------------------------
// Stream stats – running counters reset per-session
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct StreamStats {
    pub viewer_count: u32,
    pub followers_today: u32,
    pub subs_today: u32,
}

impl StreamStats {
    pub fn new() -> Self {
        Self {
            viewer_count: 0,
            followers_today: 0,
            subs_today: 0,
        }
    }

    pub fn record(&mut self, event: &StreamEvent) {
        match event {
            StreamEvent::Follow { .. } => self.followers_today += 1,
            StreamEvent::Sub { .. } | StreamEvent::GiftSub { .. } => self.subs_today += 1,
            StreamEvent::ViewerCountUpdate { count } => self.viewer_count = *count,
            _ => {}
        }
    }
}
