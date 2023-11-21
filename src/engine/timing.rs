use itertools::Itertools;
use std::collections::HashMap;
use std::time::Instant;

pub struct TimerManager {
    pub current_timers: HashMap<String, Instant>,
    pub finished_timers: HashMap<String, f32>,
}

impl TimerManager {
    pub fn new() -> Self {
        Self {
            current_timers: HashMap::new(),
            finished_timers: HashMap::new(),
        }
    }

    pub fn start<S: AsRef<str>>(&mut self, name: S) {
        self.current_timers
            .insert(name.as_ref().to_string(), Instant::now());
    }

    pub fn end<S: AsRef<str>>(&mut self, name: S) -> f32 {
        let start = self
            .current_timers
            .remove(name.as_ref())
            .expect("timer was not started yet");
        let duration = Instant::now().duration_since(start).as_secs_f32();
        self.finished_timers
            .insert(name.as_ref().to_string(), duration);

        duration
    }

    pub fn end_restart<S: AsRef<str>>(&mut self, name: S) -> f32 {
        let duration = self.end(name.as_ref());
        self.start(name.as_ref());

        duration
    }

    pub fn get_all(&self) -> Vec<(&String, f32)> {
        self.finished_timers
            .iter()
            .map(|x| (x.0, *x.1))
            .collect_vec()
    }

    pub fn clear(&mut self) {
        self.finished_timers.clear();
    }
}
