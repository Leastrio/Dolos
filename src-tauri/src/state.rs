use std::sync::atomic::AtomicBool;

pub struct DolosState {
    pub shutdown: AtomicBool
}

impl Default for DolosState {
    fn default() -> Self {
        Self { shutdown: AtomicBool::new(false) }
    }
}