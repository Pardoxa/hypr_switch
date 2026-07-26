use super::*;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug)]
pub struct Monitor {
    name: String,
    vrr: bool,
    modes: BTreeSet<MonitorMode>,
    speed_map: BTreeMap<MonitorMode, BTreeSet<MonitorSpeed>>,
}

impl Monitor {
    pub fn new(
        name: String,
        vrr: bool,
        modes: BTreeSet<MonitorMode>,
        speed_map: BTreeMap<MonitorMode, BTreeSet<MonitorSpeed>>,
    ) -> Self {
        Self {
            name,
            vrr,
            speed_map,
            modes,
        }
    }

    pub fn monitor_resolution_and_speed_string(&self, best_mode: MonitorMode) -> String {
        let speed = self.speed_map.get(&best_mode).unwrap();
        let highest_speed = speed.last().unwrap();
        format!("{best_mode}@{highest_speed}Hz")
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn is_laptop(&self) -> bool {
        self.name() == "eDP-1"
    }

    pub fn vrr(&self) -> bool {
        self.vrr
    }

    pub fn modes(&self) -> &BTreeSet<MonitorMode> {
        &self.modes
    }
}
