mod room_state;
mod sensor_memory;
mod sensor_states;

use crate::configuration::{Configuration, SensorState, SwitchState};
use crate::strategy::room_state::RoomState;
use crate::strategy::sensor_memory::SensorMemory;
use crate::strategy::sensor_states::{SensorMemoryNaiveState, SensorMemoryState};
use crate::{SensorChangeContent, SwitchChangeContent};
use std::collections::{BTreeSet, HashMap};
use std::iter::FromIterator;
use std::time::{Duration, Instant};

type Topic = String;
type Room = String;
type Sensors = HashMap<Topic, SensorMemory>;

#[derive(Debug, PartialEq)]
pub struct SwitchCommand {
    pub topic: String,
    pub state: SwitchState,
    pub brightness: u8,
}

pub struct SwitchMemory {
    pub topic: String,
    pub state: SwitchState,
    pub rooms: Vec<String>,
    pub delay: Duration,
}

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
    /// switch topics which should be permanent enabled
    enabled_switches: Vec<String>,
    /// switch topics which should be ignored
    ignored_switches: Vec<String>,

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

    /// ignore these sensors
    ignored_sensors: Vec<String>,
}

impl Strategy {
    /// create a new StateMemory object out of a Configuration
    pub fn new(configuration: &Configuration) -> Self {
        let mut room_sensors = HashMap::new();
        for sensor in configuration.sensors.iter() {
            if !room_sensors.contains_key(&sensor.room) {
                room_sensors.insert(sensor.room.clone(), HashMap::new());
            }
            let sensors_memory = room_sensors.get_mut(&sensor.room).unwrap();
            sensors_memory.insert(
                sensor.topic.clone(),
                SensorMemory {
                    delay: Duration::from_secs(sensor.delay),
                    state: SensorMemoryState::Uninitialized,
                },
            );
            info!(
                "{} contains {} with delay: {}s",
                sensor.room, sensor.topic, sensor.delay
            );
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

        let (
            brightness,
            disabled_switches,
            enabled_switches,
            ignored_switches,
            room_tracking_enabled,
            ignored_sensors,
        ) = configuration
            .scenes
            .get(0)
            .map(|default_scene| {
                (
                    default_scene.brightness.clone(),
                    default_scene.disabled_switches.clone(),
                    default_scene.enabled_switches.clone(),
                    default_scene.ignored_switches.clone(),
                    default_scene.room_tracking_enabled.clone(),
                    default_scene.ignored_sensors.clone(),
                )
            })
            .unwrap_or((255, vec![], vec![], vec![], true, vec![]));

        Strategy {
            room_sensors,
            room_switches,
            look_ahead: Duration::from_secs(look_ahead),
            room_state: HashMap::new(),
            current_room: None,
            disabled_switches,
            enabled_switches,
            ignored_switches,
            brightness,
            current_room_threshold: Duration::from_secs(current_room_threshold),
            room_tracking_enabled,
            ignored_sensors,
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

    /// trigger switch commands to set switch to expected state
    ///
    /// # Arguments
    ///
    /// * `ignore_current_state` : if set to true, all switch commands will be triggered.
    ///    if false, only states that differ current state will trigger commands
    pub fn trigger_commands(&mut self, ignore_current_state: bool) -> Vec<SwitchCommand> {
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
            } else if self.enabled_switches.contains(&switch.topic) {
                should_state = Some(On);
            } else if self.ignored_switches.contains(&switch.topic) {
                continue;
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
            if should_state.unwrap() != switch.state || ignore_current_state {
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

    pub fn set_enabled_switches(&mut self, enabled_switches: Vec<String>) {
        self.enabled_switches = enabled_switches;
    }

    pub fn set_ignored_switches(&mut self, ignored_switches: Vec<String>) {
        self.ignored_switches = ignored_switches;
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

            'room_state: for (topic, sensor_memory) in room_sensors.iter() {
                if self.ignored_sensors.contains(topic) {
                    continue;
                }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::configuration::{Credentials, Scene, Sensor};
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

    fn create_sensor(topic: &str, rooms: String, delay: u64) -> Sensor {
        Sensor {
            topic: topic.to_string(),
            key: "occupancy".to_string(),
            invert_state: false,
            delay,
            room: rooms,
        }
    }

    fn instant_from_the_past(seconds: u64) -> Instant {
        let instant = Instant::now() - Duration::from_secs(seconds);
        assert!(instant.elapsed() < Duration::from_secs(seconds + 1));
        assert!(instant.elapsed() > Duration::from_secs(seconds - 1));
        instant
    }

    fn create_test_setup() -> Strategy {
        create_test_setup_with_scene(vec![])
    }

    fn create_test_setup_with_scene(scenes: Vec<Scene>) -> Strategy {
        let configuration = Configuration {
            credentials: Credentials {
                host: "".to_string(),
                user: "".to_string(),
                password: "".to_string(),
            },
            scenes,
            sensors: vec![
                create_sensor("motion1", "room1".to_string(), 10),
                create_sensor("motion2", "room1".to_string(), 10),
            ],
            switches: vec![create_light_switch("light1", vec!["room1".to_string()])],
        };
        let strategy = Strategy::new(&configuration);

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

    #[test]
    fn test_get_room_state_ignored_sensors() {
        let scene = Scene {
            name: "test".to_string(),
            brightness: 255,
            disabled_switches: vec![],
            enabled_switches: vec![],
            ignored_switches: vec![],
            room_tracking_enabled: false,
            ignored_sensors: vec!["motion1".to_string()],
        };
        let mut strategy = create_test_setup_with_scene(vec![scene]);
        let motion_1_sensor = strategy
            .room_sensors
            .get_mut("room1")
            .unwrap()
            .get_mut("motion1");
        assert!(motion_1_sensor.is_some());
        let motion_1_sensor = motion_1_sensor.unwrap();

        motion_1_sensor.state = SensorMemoryState::Present;
        let map = strategy.get_room_state(Duration::from_secs(0));
        match map.get("room1").unwrap() {
            SensorMemoryNaiveState::Uninitialized => {}
            _ => panic!("should never happen"),
        }
    }

    #[test]
    fn test_get_room_state_ignore_all_sensors_in_room() {
        let scene = Scene {
            name: "test".to_string(),
            brightness: 255,
            disabled_switches: vec![],
            enabled_switches: vec![],
            ignored_switches: vec![],
            room_tracking_enabled: false,
            ignored_sensors: vec!["motion1".to_string(), "motion2".to_string()],
        };
        let mut strategy = create_test_setup_with_scene(vec![scene]);
        let mut room1 = strategy.room_sensors.get_mut("room1").unwrap();

        let motion_1_sensor = room1.get_mut("motion1");
        assert!(motion_1_sensor.is_some());
        let motion_1_sensor = motion_1_sensor.unwrap();
        motion_1_sensor.state = SensorMemoryState::Present;

        let motion_2_sensor = room1.get_mut("motion2");
        assert!(motion_2_sensor.is_some());
        let motion_2_sensor = motion_2_sensor.unwrap();

        motion_2_sensor.state = SensorMemoryState::Present;
        let map = strategy.get_room_state(Duration::from_secs(0));
        match map.get("room1").unwrap() {
            SensorMemoryNaiveState::Uninitialized => {}
            _ => panic!("should never happen"),
        }
    }

    #[test]
    fn test_get_room_state_ignore_one_sensors_in_room_with_presents() {
        let scene = Scene {
            name: "test".to_string(),
            brightness: 255,
            disabled_switches: vec![],
            enabled_switches: vec![],
            ignored_switches: vec![],
            room_tracking_enabled: false,
            ignored_sensors: vec!["motion1".to_string()],
        };
        let mut strategy = create_test_setup_with_scene(vec![scene]);
        let mut room1 = strategy.room_sensors.get_mut("room1").unwrap();

        let motion_1_sensor = room1.get_mut("motion1");
        assert!(motion_1_sensor.is_some());
        let motion_1_sensor = motion_1_sensor.unwrap();
        motion_1_sensor.state = SensorMemoryState::Present;

        let motion_2_sensor = room1.get_mut("motion2");
        assert!(motion_2_sensor.is_some());
        let motion_2_sensor = motion_2_sensor.unwrap();

        motion_2_sensor.state = SensorMemoryState::Present;
        let map = strategy.get_room_state(Duration::from_secs(0));
        match map.get("room1").unwrap() {
            SensorMemoryNaiveState::Present => {}
            _ => panic!("should never happen"),
        }
    }

    #[test]
    fn test_trigger_command1() {
        let mut strategy = create_test_setup();
        let commands = strategy.trigger_commands(false);
        assert!(commands.is_empty());
    }

    #[test]
    fn test_trigger_command2() {
        let mut strategy = create_test_setup();
        strategy.set_enabled_switches(vec!["zigbee2mqtt/light1".to_string()]);
        let commands = strategy.trigger_commands(false);
        assert!(!commands.is_empty());
        assert_eq!(
            commands.get(0).unwrap(),
            &SwitchCommand {
                topic: "zigbee2mqtt/light1".to_string(),
                state: SwitchState::On,
                brightness: 255,
            }
        )
    }

    #[test]
    fn test_trigger_command3() {
        let mut strategy = create_test_setup();
        strategy.set_disabled_switches(vec!["zigbee2mqtt/light1".to_string()]);
        let commands = strategy.trigger_commands(false);
        assert!(!commands.is_empty());
        assert_eq!(
            commands.get(0).unwrap(),
            &SwitchCommand {
                topic: "zigbee2mqtt/light1".to_string(),
                state: SwitchState::Off,
                brightness: 255,
            }
        )
    }

    #[test]
    fn test_trigger_command4() {
        let mut strategy = create_test_setup();
        strategy.current_room = Some("room1".to_string());
        let sensors = strategy.room_sensors.get_mut("room1").unwrap();
        sensors.get_mut("motion1").unwrap().state = SensorMemoryState::Present;
        let commands = strategy.trigger_commands(false);
        assert!(!commands.is_empty());
        assert_eq!(
            commands.get(0).unwrap(),
            &SwitchCommand {
                topic: "zigbee2mqtt/light1".to_string(),
                state: SwitchState::On,
                brightness: 255,
            }
        );
        strategy.set_ignored_switches(vec!["zigbee2mqtt/light1".to_string()]);
        let commands = strategy.trigger_commands(false);
        assert!(commands.is_empty());
    }
}
