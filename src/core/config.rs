/// Time window for trending endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TimeWindow {
    /// Results from the past day.
    Day,
    /// Results from the past week.
    Week,
}

impl std::fmt::Display for TimeWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeWindow::Day => write!(f, "day"),
            TimeWindow::Week => write!(f, "week"),
        }
    }
}
