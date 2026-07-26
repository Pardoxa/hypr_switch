use std::fmt::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MonitorSpeed {
    pub speed_low: u16,
    pub speed_high: u16,
}

impl Display for MonitorSpeed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{:02}", self.speed_high, self.speed_low)
    }
}
