use serde::export::Formatter;
use std::cmp::Ordering;
use std::time::{Duration, Instant};

#[derive(PartialEq, Debug, Clone)]
pub enum SensorMemoryNaiveState {
    /// Absent since program start
    Uninitialized,
    /// Present
    Present,
    /// was Present once but is now Absent since
    AbsentSince(Duration),
}

impl std::fmt::Display for SensorMemoryNaiveState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SensorMemoryNaiveState::Uninitialized => write!(f, "Uninitialized"),
            SensorMemoryNaiveState::Present => write!(f, "Present"),
            SensorMemoryNaiveState::AbsentSince(since) => {
                write!(f, "AbsentSince({}s)", since.as_secs())
            }
        }
    }
}
impl Ord for SensorMemoryNaiveState {
    fn cmp(&self, other: &Self) -> Ordering {
        match (&self, &other) {
            (SensorMemoryNaiveState::Present, SensorMemoryNaiveState::Present) => Ordering::Equal,
            (SensorMemoryNaiveState::AbsentSince(_), SensorMemoryNaiveState::Present) => {
                Ordering::Greater
            }
            (SensorMemoryNaiveState::Present, SensorMemoryNaiveState::AbsentSince(_)) => {
                Ordering::Less
            }
            (SensorMemoryNaiveState::AbsentSince(me), SensorMemoryNaiveState::AbsentSince(it)) => {
                me.cmp(&it)
            }
            (SensorMemoryNaiveState::Uninitialized, SensorMemoryNaiveState::Uninitialized) => {
                Ordering::Equal
            }
            (SensorMemoryNaiveState::Uninitialized, SensorMemoryNaiveState::Present) => {
                Ordering::Greater
            }
            (SensorMemoryNaiveState::Uninitialized, SensorMemoryNaiveState::AbsentSince(_)) => {
                Ordering::Greater
            }
            (SensorMemoryNaiveState::Present, SensorMemoryNaiveState::Uninitialized) => {
                Ordering::Less
            }
            (SensorMemoryNaiveState::AbsentSince(_), SensorMemoryNaiveState::Uninitialized) => {
                Ordering::Less
            }
        }
    }
}
impl PartialOrd for SensorMemoryNaiveState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Eq for SensorMemoryNaiveState {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order() {
        assert!(SensorMemoryNaiveState::Present < SensorMemoryNaiveState::Uninitialized);
        assert!(
            SensorMemoryNaiveState::Present
                < SensorMemoryNaiveState::AbsentSince(Duration::from_secs(0))
        );
        assert!(
            SensorMemoryNaiveState::Present
                < SensorMemoryNaiveState::AbsentSince(Duration::from_secs(10))
        );
        assert!(
            SensorMemoryNaiveState::AbsentSince(Duration::from_secs(10))
                < SensorMemoryNaiveState::AbsentSince(Duration::from_secs(20))
        );
    }
}

#[derive(PartialEq, Debug, Clone)]
pub enum SensorMemoryState {
    /// Absent since program start
    Uninitialized,
    /// Present
    Present,
    /// was Present once but is now Absent since
    AbsentSince(Instant),
}

impl std::fmt::Display for SensorMemoryState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SensorMemoryState::Uninitialized => write!(f, "Uninitialized"),
            SensorMemoryState::Present => write!(f, "Present"),
            SensorMemoryState::AbsentSince(since) => {
                write!(f, "AbsentSince({}s)", since.elapsed().as_secs())
            }
        }
    }
}
