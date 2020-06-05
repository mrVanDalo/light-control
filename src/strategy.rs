mod sensor_states;

use crate::configuration::{Configuration, SensorState, SwitchState};
use crate::strategy::sensor_states::{SensorMemoryNaiveState, SensorMemoryState};
use crate::{SensorChangeContent, SwitchChangeContent};
use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap};
use std::iter::FromIterator;
use std::time::{Duration, Instant};

type Topic = String;
type Room = String;
type Sensors = HashMap<Topic, SensorMemory>;

pub struct Strategy {
    /// all known sensors grouped room
    room_sensors: HashMap<Room, Sensors>,

    /// all known switches grouped room
    room_switches: Vec<SwitchMemory>,

    /// room state cache to print nice messages
    room_state: HashMap<Room, SensorMemoryNaiveState>,

    /// room we think the user is located
    current_room: Option<Room>,

    /// switch topics which should be permanent disabled
    disabled_switches: Vec<String>,

    /// current brightness
    brightness: u8,

    /// min possible delay of all sensors, to look in the future and
    /// determine the current_room
    look_ahead: Duration,

    /// threshold for current room determination.
    /// If a new room is shorter absent than the current room
    /// it must be shorter by the factor of this factor
    current_room_threshold: Duration,

    /// weather or not current_room should stay on or not
    room_tracking_enabled: bool,
}

impl Strategy {
    /// create a new StateMemory object out of a Configuration
    pub fn new(configuration: &Configuration) -> Self {
        let mut room_sensors = HashMap::new();
        for sensor in configuration.sensors.iter() {
            for room in sensor.rooms.iter() {
                if !room_sensors.contains_key(room) {
                    room_sensors.insert(room.clone(), HashMap::new());
                }
                let sensors_memory = room_sensors.get_mut(room).unwrap();
                sensors_memory.insert(
                    sensor.topic.clone(),
                    SensorMemory {
                        delay: Duration::from_secs(sensor.delay),
                        state: SensorMemoryState::Uninitialized,
                    },
                );
                info!(
                    "{} contains {} with delay: {}s",
                    room, sensor.topic, sensor.delay
                );
            }
        }
        let mut room_switches = Vec::new();
        for switch in configuration.switches.iter() {
            room_switches.push(SwitchMemory {
                topic: switch.topic.clone(),
                state: SwitchState::Unknown,
                rooms: switch.rooms.clone(),
                delay: Duration::from_secs(switch.delay),
            });
        }
        let look_ahead = configuration.get_min_sensor_delay();
        if look_ahead < 10 {
            warn!("warning: you have configured a sensor delay below 10 seconds, this can cause wrong location calculation");
        }
        info!("look ahead: {}s", look_ahead);
        let current_room_threshold = look_ahead / 2;
        info!("current room threshold: {}s", current_room_threshold);
        if look_ahead < current_room_threshold {
            warn!("look ahead is smaller than current room threshold, lights will be turned off before current room detections is calculated")
        }

        let (brightness, disabled_switches, room_tracking_enabled) = configuration
            .scenes
            .get(0)
            .map(|default_scene| {
                (
                    default_scene.brightness.clone(),
                    default_scene.exclude_switches.clone(),
                    default_scene.room_tracking_enabled.clone(),
                )
            })
            .unwrap_or((255, vec![], true));

        Strategy {
            room_sensors,
            room_switches,
            look_ahead: Duration::from_secs(look_ahead),
            room_state: HashMap::new(),
            current_room: None,
            disabled_switches,
            brightness,
            current_room_threshold: Duration::from_secs(current_room_threshold),
            room_tracking_enabled,
        }
    }

    /// after some time none of the sensors can stay on the Initialized state
    pub fn replace_uninitialized_with_absents(&mut self, instant: Instant) {
        info!("takeover: all uninitialized sensors set to absent and all uninitialized switches will be turned off");
        for sensor in self.room_sensors.values_mut() {
            for sensor_state in sensor.values_mut() {
                if sensor_state.state == SensorMemoryState::Uninitialized {
                    sensor_state.state = SensorMemoryState::AbsentSince(instant.clone());
                }
            }
        }
    }

    pub fn update_sensor(&mut self, instant: Instant, sensor_content: SensorChangeContent) {
        for room in self.room_sensors.values_mut() {
            room.get_mut(&sensor_content.topic).map(|sensor_memory| {
                match (&sensor_memory.state, sensor_content.state) {
                    (SensorMemoryState::Uninitialized, SensorState::Absent) => {
                        sensor_memory.state = SensorMemoryState::AbsentSince(instant);
                    }
                    (SensorMemoryState::Uninitialized, SensorState::Present) => {
                        sensor_memory.state = SensorMemoryState::Present
                    }
                    (SensorMemoryState::AbsentSince(_), SensorState::Absent) => (),
                    (SensorMemoryState::AbsentSince(_), SensorState::Present) => {
                        sensor_memory.state = SensorMemoryState::Present
                    }
                    (SensorMemoryState::Present, SensorState::Absent) => {
                        sensor_memory.state = SensorMemoryState::AbsentSince(instant)
                    }
                    (SensorMemoryState::Present, SensorState::Present) => (),
                }
            });
        }
    }

    pub fn update_switch(&mut self, _instant: Instant, switch_content: SwitchChangeContent) {
        for mut room_switch in self.room_switches.iter_mut() {
            if room_switch.topic != switch_content.topic {
                continue;
            }
            room_switch.state = switch_content.state;
            break;
        }
    }

    pub fn calculate_current_room(&mut self) {
        let rooms = self.get_room_state(self.look_ahead);
        // prepare sorted_rooms list
        let mut sorted_rooms = BTreeSet::new();
        for (room, sensor_state) in rooms.iter() {
            sorted_rooms.insert(RoomState {
                room: room.clone(),
                state: sensor_state.clone(),
            });
        }
        let sorted_rooms: Vec<&RoomState> = Vec::from_iter(sorted_rooms.iter());
        if sorted_rooms.get(1).is_none() {
            //debug!("because only one room is known current_room tracking is disabled");
            self.current_room = None;
            return;
        }
        if sorted_rooms.get(1).unwrap().state == SensorMemoryNaiveState::Present {
            // to much rooms are present
            return;
        }
        if sorted_rooms.is_empty() {
            //debug!("because no rooms are defined, current_room tracking is disabled");
            return;
        }
        if sorted_rooms.get(0).unwrap().state == SensorMemoryNaiveState::Present {
            let current_room = sorted_rooms
                .get(0)
                .map(|room_state| room_state.room.clone());
            if current_room == self.current_room {
                return;
            }
            self.current_room = current_room;
            debug!(
                "because one room is present and all other rooms are absent, current_room : {:?}",
                self.current_room
            );
            return;
        }
        if sorted_rooms.get(0).unwrap().state == SensorMemoryNaiveState::Uninitialized {
            self.current_room = None;
            //debug!("because all rooms are uninitialized no current_room is defined");
            return;
        }
        if self.current_room.is_none() {
            self.current_room = sorted_rooms
                .get(0)
                .map(|room_state| room_state.room.clone());
            debug!(
                "because no current_room is defined, we use the room with the lowest absents: {:?}",
                self.current_room
            );
            return;
        }
        let current_room = self.current_room.clone().unwrap();
        let mut room_compare_index = 1;
        if sorted_rooms.get(0).unwrap().room != current_room {
            room_compare_index = 0;
        }
        match (
            &sorted_rooms.get(room_compare_index).unwrap().state,
            rooms.get(&current_room).unwrap(),
        ) {
            (SensorMemoryNaiveState::Uninitialized, _) => {}
            (SensorMemoryNaiveState::AbsentSince(_), SensorMemoryNaiveState::Uninitialized) => {
                self.current_room = sorted_rooms
                    .get(room_compare_index)
                    .map(|room_state| room_state.room.clone());
                debug!(
                    "because current_room uninitialized, new current room is : {:?}",
                    self.current_room
                );
                return;
            }
            (SensorMemoryNaiveState::AbsentSince(_), SensorMemoryNaiveState::Present) => {}
            (
                SensorMemoryNaiveState::AbsentSince(other_room_duration),
                SensorMemoryNaiveState::AbsentSince(current_room_duration),
            ) => {
                if other_room_duration > current_room_duration {
                    // current_room is still shorter absent
                    return;
                }
                if current_room_duration > &self.current_room_threshold {
                    self.current_room = sorted_rooms
                        .get(room_compare_index)
                        .map(|room_state| room_state.room.clone());
                    debug!("because current_room is longer absent than another room new current room is : {:?}", self.current_room);
                    return;
                }
            }
            // should never happen
            (SensorMemoryNaiveState::Present, _) => {}
        }
    }

    /// find situation where a switch has a state it shouldn't have
    /// and create command to correct that
    pub fn trigger_commands(&mut self) -> Vec<SwitchCommand> {
        let new_room_states = self.get_room_state(Duration::from_secs(0));
        Strategy::print_room_update_information(&new_room_states, &self.room_state);
        self.room_state = new_room_states;

        // update commands
        let mut commands = Vec::new();
        for switch in self.room_switches.iter() {
            use SwitchState::{Off, On};
            let mut should_state = None;
            if self.disabled_switches.contains(&switch.topic) {
                should_state = Some(Off);
            } else {
                'find_should_state: for room in switch.rooms.iter() {
                    if Some(room) == self.current_room.as_ref() && self.room_tracking_enabled {
                        should_state = Some(On);
                        break 'find_should_state;
                    }
                    match &self.room_state.get(room).unwrap() {
                        SensorMemoryNaiveState::Present => {
                            should_state = Some(On);
                            break 'find_should_state;
                        }
                        SensorMemoryNaiveState::AbsentSince(duration) => {
                            if duration > &switch.delay {
                                trace!(
                                    "{} with delay {}s is - Off- because of AbsentSince({}s)",
                                    switch.topic,
                                    switch.delay.as_secs(),
                                    duration.as_secs()
                                );
                                should_state = Some(Off);
                            } else {
                                trace!(
                                    "{} with delay {}s is - ON - because of AbsentSince({}s)",
                                    switch.topic,
                                    switch.delay.as_secs(),
                                    duration.as_secs()
                                );
                                should_state = Some(On);
                            }
                        }
                        _ => {}
                    }
                }
            }
            if should_state.is_none() {
                continue;
            }
            if should_state.unwrap() != switch.state {
                trace!("set {} -> {:?}", switch.topic, should_state.unwrap());
                commands.push(SwitchCommand {
                    topic: switch.topic.clone(),
                    state: should_state.unwrap(),
                    brightness: self.brightness,
                })
            }
        }
        commands
    }

    fn print_room_update_information(
        new_room_states: &HashMap<String, SensorMemoryNaiveState>,
        current_room_states: &HashMap<String, SensorMemoryNaiveState>,
    ) {
        for (room, new_state) in new_room_states.iter() {
            let old_state = current_room_states.get(room);
            if old_state.is_none() {
                continue;
            }
            let old_state = old_state.unwrap();
            match (old_state, new_state) {
                (SensorMemoryNaiveState::Uninitialized, SensorMemoryNaiveState::Uninitialized) => {}
                (SensorMemoryNaiveState::Present, SensorMemoryNaiveState::Present) => {}
                (
                    SensorMemoryNaiveState::AbsentSince(_),
                    SensorMemoryNaiveState::AbsentSince(_),
                ) => {}
                _ => {
                    trace!("realized {} changed {} -> {}", room, old_state, new_state,);
                }
            }
        }
    }

    pub fn set_brightness(&mut self, brightness: u8) {
        self.brightness = brightness;
    }

    pub fn set_room_tracking_enabled(&mut self, room_tracking_enabled: bool) {
        self.room_tracking_enabled = room_tracking_enabled;
    }

    pub fn set_disabled_switches(&mut self, disabled_switches: Vec<String>) {
        self.disabled_switches = disabled_switches;
    }

    /// the current state of the room.
    /// sensor delays are taken into account
    ///
    /// # Arguments
    ///
    /// * `look_ahead` - look ahead in the future
    ///
    fn get_room_state(&self, look_ahead: Duration) -> HashMap<String, SensorMemoryNaiveState> {
        let mut rooms = HashMap::new();

        for (room, room_sensors) in self.room_sensors.iter() {
            let mut current_room_state: SensorMemoryNaiveState =
                SensorMemoryNaiveState::Uninitialized;

            'room_state: for (_topic, sensor_memory) in room_sensors.iter() {
                match (
                    &current_room_state,
                    &sensor_memory.get_naive_state(look_ahead),
                ) {
                    (_, SensorMemoryNaiveState::Uninitialized) => {}

                    (_, SensorMemoryNaiveState::Present) => {
                        current_room_state = SensorMemoryNaiveState::Present;
                        break 'room_state;
                    }

                    (
                        SensorMemoryNaiveState::AbsentSince(current_duration),
                        SensorMemoryNaiveState::AbsentSince(new_duration),
                    ) => {
                        if current_duration > new_duration {
                            current_room_state =
                                SensorMemoryNaiveState::AbsentSince(new_duration.clone());
                        }
                    }

                    (
                        SensorMemoryNaiveState::Uninitialized,
                        SensorMemoryNaiveState::AbsentSince(duration),
                    ) => {
                        current_room_state = SensorMemoryNaiveState::AbsentSince(duration.clone());
                    }

                    (
                        SensorMemoryNaiveState::Present,
                        SensorMemoryNaiveState::AbsentSince(duration),
                    ) => {
                        current_room_state = SensorMemoryNaiveState::AbsentSince(duration.clone());
                    }
                };
            }
            rooms.insert(room.clone(), current_room_state);
        }
        rooms
    }
}

/// Sorting structure for room state
#[derive(Debug)]
pub struct RoomState {
    room: String,
    state: SensorMemoryNaiveState,
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
    use std::ops::Bound::Included;

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

pub struct SwitchCommand {
    pub topic: String,
    pub state: SwitchState,
    pub brightness: u8,
}

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

pub struct SwitchMemory {
    pub topic: String,
    pub state: SwitchState,
    pub rooms: Vec<String>,
    pub delay: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::configuration::{Credentials, Sensor};
    use crate::dummy_configuration::create_light_switch;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_instant_difference() {
        let instant1 = Instant::now();
        thread::sleep(Duration::new(2, 0));
        let instant2 = Instant::now();
        thread::sleep(Duration::new(2, 0));
        assert!(instant1.elapsed() > instant2.elapsed())
    }

    #[test]
    fn test_instant_arithmetic() {
        let instant = instant_from_the_past(20);
        assert!(instant.elapsed() - Duration::from_secs(10) > Duration::from_secs(10));
        assert!(instant.elapsed() - Duration::from_secs(10) < Duration::from_secs(11));
    }

    fn create_sensor(topic: &str, rooms: Vec<String>, delay: u64) -> Sensor {
        Sensor {
            topic: topic.to_string(),
            key: "occupancy".to_string(),
            invert_state: false,
            delay,
            rooms,
        }
    }

    fn instant_from_the_past(seconds: u64) -> Instant {
        let instant = Instant::now() - Duration::from_secs(seconds);
        assert!(instant.elapsed() < Duration::from_secs(seconds + 1));
        assert!(instant.elapsed() > Duration::from_secs(seconds - 1));
        instant
    }

    fn create_test_setup() -> Strategy {
        let configuration = Configuration {
            credentials: Credentials {
                host: "".to_string(),
                user: "".to_string(),
                password: "".to_string(),
            },
            scenes: vec![],
            sensors: vec![
                create_sensor("motion1", vec!["room1".to_string()], 10),
                create_sensor("motion2", vec!["room1".to_string()], 10),
            ],
            switches: vec![create_light_switch("light1", vec!["room1".to_string()])],
        };
        let mut strategy = Strategy::new(&configuration);

        // test if sensors are proper initialized
        let map = strategy.get_room_state(Duration::from_secs(0));
        assert!(map.get("room1").is_some());
        assert_eq!(
            &SensorMemoryNaiveState::Uninitialized,
            map.get("room1").unwrap(),
            "room1 is not uninitialised"
        );
        strategy
    }

    #[test]
    fn test_get_room_state_absent() {
        let mut strategy = create_test_setup();
        let motion_1_sensor = strategy
            .room_sensors
            .get_mut("room1")
            .unwrap()
            .get_mut("motion1");
        assert!(motion_1_sensor.is_some());
        let motion_1_sensor = motion_1_sensor.unwrap();

        let instant = instant_from_the_past(60);
        motion_1_sensor.state = SensorMemoryState::AbsentSince(instant);
        let map = strategy.get_room_state(Duration::from_secs(0));
        match map.get("room1").unwrap() {
            SensorMemoryNaiveState::AbsentSince(duration) => {
                assert!(duration < &Duration::from_secs(51));
                assert!(duration > &Duration::from_secs(50));
            }
            _ => panic!("should never happen"),
        }
    }

    #[test]
    fn test_get_room_state_absent_and_delay() {
        let mut strategy = create_test_setup();
        let motion_1_sensor = strategy
            .room_sensors
            .get_mut("room1")
            .unwrap()
            .get_mut("motion1");
        assert!(motion_1_sensor.is_some());
        let motion_1_sensor = motion_1_sensor.unwrap();

        let instant = instant_from_the_past(2);
        motion_1_sensor.state = SensorMemoryState::AbsentSince(instant);
        let map = strategy.get_room_state(Duration::from_secs(0));
        assert_eq!(&SensorMemoryNaiveState::Present, map.get("room1").unwrap());
    }

    #[test]
    fn test_get_room_state_absent_and_delay_with_previous_state() {
        let mut strategy = create_test_setup();
        // setting the room_state to be offline for about 5 secs delay is already included here
        strategy.room_state.insert(
            "room1".to_string(),
            SensorMemoryNaiveState::AbsentSince(Duration::from_secs(5)),
        );
        let motion_1_sensor = strategy
            .room_sensors
            .get_mut("room1")
            .unwrap()
            .get_mut("motion1");
        assert!(motion_1_sensor.is_some());
        let motion_1_sensor = motion_1_sensor.unwrap();

        let instant = instant_from_the_past(2);
        motion_1_sensor.state = SensorMemoryState::AbsentSince(instant);
        let map = strategy.get_room_state(Duration::from_secs(0));
        assert_eq!(&SensorMemoryNaiveState::Present, map.get("room1").unwrap());
    }

    #[test]
    fn test_get_room_state_present() {
        let mut strategy = create_test_setup();
        let motion_1_sensor = strategy
            .room_sensors
            .get_mut("room1")
            .unwrap()
            .get_mut("motion1");
        assert!(motion_1_sensor.is_some());
        let motion_1_sensor = motion_1_sensor.unwrap();

        let instant = instant_from_the_past(12);
        motion_1_sensor.state = SensorMemoryState::Present;
        let map = strategy.get_room_state(Duration::from_secs(0));
        assert_eq!(&SensorMemoryNaiveState::Present, map.get("room1").unwrap());
    }
}
