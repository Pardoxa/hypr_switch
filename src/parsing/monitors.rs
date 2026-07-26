use crate::domain::*;
use std::collections::{BTreeMap, BTreeSet};

pub fn parse_monitors(to_parse: &str) -> Vec<Monitor> {
    let monitors = to_parse.split("Monitor ").skip(1);

    let monitors: Vec<_> = monitors
        .map(|monitor| {
            let (monitor_name, rest) = monitor.split_once(" ").unwrap();
            let mut vrr = false;
            let mut modes: BTreeSet<MonitorMode> = BTreeSet::new();
            let mut speed_map = BTreeMap::new();
            for line in rest.lines() {
                let line = line.trim_start();
                if let Some(vrr_str) = line.strip_prefix("vrr: ") {
                    vrr = vrr_str.parse().unwrap_or(false);
                    continue;
                }
                if let Some(available_modes) = line.strip_prefix("availableModes: ") {
                    for mode in available_modes.split_ascii_whitespace() {
                        let (mode, speed) = monitor_mode_and_speed(mode);
                        modes.insert(mode);

                        speed_map
                            .entry(mode)
                            .or_insert_with(BTreeSet::new)
                            .insert(speed);
                    }
                }
            }
            let name = monitor_name.to_owned();
            Monitor::new(name, vrr, modes, speed_map)
        })
        .collect();
    monitors
}

fn monitor_mode_and_speed(s: &str) -> (MonitorMode, MonitorSpeed) {
    let (x, rest) = s.split_once('x').unwrap();
    let (y, rest) = rest.split_once('@').unwrap();

    let (speed_high, rest) = rest.split_once('.').unwrap();
    let speed_low = rest.strip_suffix("Hz").unwrap();

    (
        MonitorMode {
            x: x.parse().unwrap(),
            y: y.parse().unwrap(),
        },
        MonitorSpeed {
            speed_high: speed_high.parse().unwrap(),
            speed_low: speed_low.parse().unwrap(),
        },
    )
}
