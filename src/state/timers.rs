use std::time::{Duration, Instant};

pub struct PollingTimers {
    pub planet_snapshot_timer: Instant,
    pub planet_snapshot_interval: Duration,
    pub explorer_snapshot_timer: Instant,
    pub explorer_snapshot_interval: Duration,
    pub explorer_position_timer: Instant,
    pub explorer_position_interval: Duration,
}

impl PollingTimers {
    pub fn new() -> Self {
        Self {
            planet_snapshot_timer: Instant::now(),
            planet_snapshot_interval: Duration::from_millis(200),
            explorer_snapshot_timer: Instant::now(),
            explorer_snapshot_interval: Duration::from_millis(200),
            explorer_position_timer: Instant::now(),
            explorer_position_interval: Duration::from_millis(200),
        }
    }

    /// Returns `true` (and resets the timer) when it is time to poll planet snapshots.
    pub fn should_poll_planet_snapshots(&mut self) -> bool {
        if self.planet_snapshot_timer.elapsed() >= self.planet_snapshot_interval {
            self.planet_snapshot_timer = Instant::now();
            true
        } else {
            false
        }
    }

    /// Returns `true` (and resets the timer) when it is time to poll explorer snapshots.
    pub fn should_poll_explorer_snapshots(&mut self) -> bool {
        if self.explorer_snapshot_timer.elapsed() >= self.explorer_snapshot_interval {
            self.explorer_snapshot_timer = Instant::now();
            true
        } else {
            false
        }
    }

    /// Returns `true` (and resets the timer) when it is time to poll explorer positions.
    pub fn should_poll_explorer_positions(&mut self) -> bool {
        if self.explorer_position_timer.elapsed() >= self.explorer_position_interval {
            self.explorer_position_timer = Instant::now();
            true
        } else {
            false
        }
    }
}
