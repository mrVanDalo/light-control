use crate::strategy::sensor_states::{SensorMemoryNaiveState, SensorMemoryState};
use std::time::Duration;

// todo: rename it
pub struct SensorMemory {
    pub delay: Duration,
    pub state: SensorMemoryState,
}

impl SensorMemory {
    pub fn get_naive_state(&self, look_ahead: Duration) -> SensorMemoryNaiveState {
        match self.state {
            SensorMemoryState::Uninitialized => SensorMemoryNaiveState::Uninitialized,
            SensorMemoryState::Present => SensorMemoryNaiveState::Present,
            SensorMemoryState::AbsentSince(instant) => {
                let duration = instant.elapsed() + look_ahead;
                if duration < self.delay {
                    SensorMemoryNaiveState::Present
                } else {
                    SensorMemoryNaiveState::AbsentSince(duration - self.delay)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests_sensor_memory {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_get_naive_state_1() {
        let instant = Instant::now() - Duration::from_secs(30);
        let sensor_memory = SensorMemory {
            delay: Duration::from_secs(60),
            state: SensorMemoryState::AbsentSince(instant),
        };
        assert_eq!(
            sensor_memory.get_naive_state(Duration::from_secs(0)),
            SensorMemoryNaiveState::Present
        );
    }

    #[test]
    fn test_get_naive_state_2() {
        let instant = Instant::now() - Duration::from_secs(62);
        let sensor_memory = SensorMemory {
            delay: Duration::from_secs(60),
            state: SensorMemoryState::AbsentSince(instant),
        };
        let naive_state = sensor_memory.get_naive_state(Duration::from_secs(0));
        assert_ne!(naive_state, SensorMemoryNaiveState::Present,);
        assert_ne!(naive_state, SensorMemoryNaiveState::Uninitialized,);
        match naive_state {
            SensorMemoryNaiveState::AbsentSince(duration) => {
                assert!(duration < Duration::from_secs(3));
                assert!(duration > Duration::from_secs(2));
            }
            _ => panic!("never gonna happen"),
        }
    }
}
