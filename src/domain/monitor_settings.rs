use std::fmt::Display;

use super::*;

pub struct MonitorSettings {
    output: String,
    mode: String,
    position: String,
    scale: String,
    transform: u8,
    bitdepth: Option<u8>,
    mirror: Option<String>,
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
        writeln!(
            f,
            "monitorv2 {{\n\toutput = {}\n\tmode = {}\n\tposition = {}\n\tscale = {}\n\ttransform = {}",
            self.output, self.mode, self.position, self.scale, self.transform
        )?;
        if self.vrr {
            writeln!(f, "\tvrr = 1")?;
        }
        if let Some(bit_depth) = self.bitdepth {
            writeln!(f, "\tbitdepth = {bit_depth}")?;
        }
        if let Some(mirror) = self.mirror.as_deref() {
            writeln!(f, "\tmirror = {mirror}")?;
        }
        writeln!(f, "}}")
    }
}
