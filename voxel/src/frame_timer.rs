use std::time::{Duration, Instant};

pub struct FrameTimer {
    last_frame: Instant,
}

impl FrameTimer {
    pub fn new() -> Self {
        Self {
            last_frame: Instant::now(),
        }
    }

    pub fn get_dt(&mut self) -> Duration {
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame);

        self.last_frame = now;

        dt
    }
}
