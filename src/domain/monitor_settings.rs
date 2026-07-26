use std::fmt::Display;

use super::*;

pub struct MonitorSettings {
    output: String,
    mode: String,
    position: String,
    scale: String,
    #[allow(dead_code)]
    transform: u8,
    bitdepth: Option<u8>,
    mirror: Option<String>,
    #[allow(dead_code)]
    vrr: bool,
}

impl MonitorSettings {
    /// The additional offset for the next monitor
    pub fn offset_for_next_monitor(&self) -> u32 {
        if let Some((x, _)) = self.mode.trim_start().split_once('x') {
            x.parse().expect("unable to parse x coordinate")
        } else {
            panic!("No x coordinate found");
        }
    }

    /// Sets an offset where y is not offset
    /// and x is offset by the given amount
    pub fn set_position_offset_x(&mut self, x: u32) {
        self.position = format!("{x}x0");
    }

    pub fn new(monitor: &Monitor, best_mode: MonitorMode) -> Self {
        // For now, I only put it in for my laptop screen, I may update this in the future
        let bitdepth = if monitor.name() == "eDP-1" {
            Some(10)
        } else {
            None
        };
        Self {
            mirror: None,
            mode: monitor.monitor_resolution_and_speed_string(best_mode),
            vrr: monitor.vrr(),
            output: monitor.name().to_owned(),
            scale: "1".to_owned(),
            position: "0x0".to_owned(),
            transform: 0, // No transform for now
            bitdepth,
        }
    }

    pub fn set_scale(&mut self, scale: String) {
        self.scale = scale;
    }

    pub fn mirror(&mut self, monitor: &Monitor) {
        self.mirror = Some(monitor.name().to_owned());
    }
}

impl Display for MonitorSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let bitdepth = self
            .bitdepth
            .map(|bitdepth| format!(r#"bitdepth = "{}","#, bitdepth))
            .unwrap_or_default();
        // TODO: VRR
        // TODO: Mirror
        writeln!(
            f,
            r#"hl.monitor({{
    output = "{}",
    mode = "{}",
    position = "{}",
    scale = "{}",
    {}
}})"#,
            self.output, self.mode, self.position, self.scale, bitdepth
        )
    }
}
