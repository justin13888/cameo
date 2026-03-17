/// Time window for trending endpoints.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::TimeWindow;

    #[test]
    fn time_window_as_hashmap_key() {
        let mut map = HashMap::new();
        map.insert(TimeWindow::Day, "daily");
        map.insert(TimeWindow::Week, "weekly");
        assert_eq!(map[&TimeWindow::Day], "daily");
        assert_eq!(map[&TimeWindow::Week], "weekly");
    }
}
