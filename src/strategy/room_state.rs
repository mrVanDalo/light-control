use crate::strategy::sensor_states::SensorMemoryNaiveState;
use std::cmp::Ordering;

/// Sorting structure for room state
#[derive(Debug)]
pub struct RoomState {
    pub room: String,
    pub state: SensorMemoryNaiveState,
}
impl Ord for RoomState {
    /// If duration is not set, it means it is present
    fn cmp(&self, other: &Self) -> Ordering {
        match &self.state.cmp(&other.state) {
            Ordering::Equal => self.room.cmp(&other.room),
            otherwise => otherwise.clone(),
        }
    }
}
impl PartialOrd for RoomState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl PartialEq for RoomState {
    fn eq(&self, other: &Self) -> bool {
        self.room.eq(&other.room)
    }
}
impl Eq for RoomState {}

#[cfg(test)]
mod test_room_absents {
    use super::*;

    use std::collections::BTreeSet;
    use std::iter::FromIterator;
    use std::time::Duration;

    #[test]
    fn test_room_absent_order() {
        let mut set = BTreeSet::new();
        set.insert(RoomState {
            room: "test1".to_string(),
            state: SensorMemoryNaiveState::AbsentSince(Duration::from_secs(20)),
        });
        set.insert(RoomState {
            room: "test2".to_string(),
            state: SensorMemoryNaiveState::Present,
        });
        set.insert(RoomState {
            room: "test3".to_string(),
            state: SensorMemoryNaiveState::AbsentSince(Duration::from_secs(100)),
        });
        set.insert(RoomState {
            room: "test4".to_string(),
            state: SensorMemoryNaiveState::Present,
        });
        set.insert(RoomState {
            room: "test5".to_string(),
            state: SensorMemoryNaiveState::Uninitialized,
        });
        set.insert(RoomState {
            room: "test6".to_string(),
            state: SensorMemoryNaiveState::AbsentSince(Duration::from_secs(2)),
        });
        let vec: Vec<&RoomState> = Vec::from_iter(set.iter());
        assert_eq!(vec.get(0).unwrap().state, SensorMemoryNaiveState::Present);
        assert_eq!(vec.get(1).unwrap().state, SensorMemoryNaiveState::Present);
        assert_eq!(
            vec.get(2).unwrap().state,
            SensorMemoryNaiveState::AbsentSince(Duration::from_secs(2))
        );
        assert_eq!(
            vec.get(3).unwrap().state,
            SensorMemoryNaiveState::AbsentSince(Duration::from_secs(20))
        );
        assert_eq!(
            vec.get(4).unwrap().state,
            SensorMemoryNaiveState::AbsentSince(Duration::from_secs(100))
        );
        assert_eq!(
            vec.get(5).unwrap().state,
            SensorMemoryNaiveState::Uninitialized
        );
    }
}
