use crate::configuration::{Configuration, SensorState, SwitchState};
use crate::strategy::SensorMemoryState::{AbsentSince, Present, Uninitialized};
use crate::{SensorChangeContent, SwitchChangeContent};
use std::collections::HashMap;
use std::time::{Duration, Instant};

type Topic = String;
type Room = String;
type Sensors = HashMap<Topic, SensorMemory>;

pub struct Strategy {
    initialisation_time: Instant,

    /// all known sensors grouped room
    room_sensors: HashMap<Room, Sensors>,

    /// all known switches grouped room
    room_switches: Vec<SwitchMemory>,

    /// room state cache to print nice messages
    room_state: HashMap<Room, SensorMemoryState>,

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
        let mut look_ahead = 300;
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
            if sensor.delay < look_ahead {
                look_ahead = sensor.delay;
            }
        }
        let mut room_switches = Vec::new();
        for switch in configuration.switches.iter() {
            room_switches.push(SwitchMemory {
                topic: switch.topic.clone(),
                state: SwitchState::Unknown,
                rooms: switch.rooms.clone(),
            });
        }
        if look_ahead < 10 {
            warn!("warning: you have configured a sensor delay below 10 seconds, this can cause wrong location calculation");
        }
        info!("look ahead: {}s", look_ahead);
        let current_room_threshold = look_ahead / 2;
        info!("current room threshold: {}s", current_room_threshold);
        if look_ahead < current_room_threshold {
            warn!("look ahead is smaller than current room threshold, lights will be turned off before current room detections is calculated")
        }

        let (brightness, disabled_switches) = configuration
            .scenes
            .get(0)
            .map(|default_scene| {
                (
                    default_scene.brightness.clone(),
                    default_scene.exclude_switches.clone(),
                )
            })
            .unwrap_or((255, vec![]));

        Strategy {
            initialisation_time: Instant::now(),
            room_sensors,
            room_switches,
            look_ahead: Duration::from_secs(look_ahead),
            room_state: HashMap::new(),
            current_room: None,
            disabled_switches,
            brightness,
            current_room_threshold: Duration::from_secs(current_room_threshold),
            room_tracking_enabled: true,
        }
    }

    /// after some time none of the sensors can stay on the Initialized state
    pub fn replace_uninitialized_with_absents(&mut self, instant: Instant) {
        info!("takeover: all uninitialized sensors set to absent and all uninitialized switches will be turned off");
        for sensor in self.room_sensors.values_mut() {
            for sensor_state in sensor.values_mut() {
                if sensor_state.state == Uninitialized {
                    sensor_state.state = AbsentSince(instant.clone());
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

        let mut current_room_absents = Duration::from_secs(60 * 55); // todo use Option here
        let mut youngest_absents = Duration::from_secs(60 * 60); // todo use Option here
        debug_assert!(youngest_absents + self.current_room_threshold > current_room_absents);

        let mut youngest_room = "".to_string();
        let mut present_counter = 0;
        let mut present_room = "".to_string();

        for (room, state) in rooms {
            match state {
                Present => {
                    present_counter = present_counter + 1;
                    present_room = room;

                    if present_counter > 1 {
                        // to much rooms detected presents
                        return;
                    };
                }
                AbsentSince(instant) => {
                    if self.current_room.is_some() {
                        if self.current_room.as_ref().unwrap() == &room {
                            current_room_absents = instant.elapsed();
                        }
                    }
                    if instant.elapsed() < youngest_absents {
                        youngest_absents = instant.elapsed();
                        youngest_room = room;
                    }
                }
                Uninitialized => {}
            }
        }

        if present_counter == 1 {
            if self.current_room.is_none() {
                self.current_room = Some(present_room);
                debug!(
                    "because of single presents, current_room is set to : {}",
                    self.current_room.as_ref().unwrap()
                );
                return;
            }
            if self.current_room.as_ref().unwrap() == &present_room {
                return;
            }
            // current_room_absents needs to be set now
            if current_room_absents < self.current_room_threshold {
                return;
            }
            debug!(
                "because current_room is to long absent ({} - {}s), new current_room is set to : {}",
                self.current_room.as_ref().unwrap_or(&"current_room not set yet".to_string()),
                current_room_absents.as_secs(),
                present_room
            );
            self.current_room = Some(present_room);
            return;
        }

        if self.current_room.is_none() {
            // don't compare the longest absence since if no presents was ever detected
            return;
        }
        if youngest_absents + self.current_room_threshold < current_room_absents {
            debug!(
                "because of current_room ({} - {}s) is longer absent than another room ({}s), current_room is set to : {}",
                self.current_room.as_ref().unwrap_or(&"---".to_string()),
                current_room_absents.as_secs(),
                youngest_absents.as_secs(),
                youngest_room
            );
            self.current_room = Some(youngest_room);
            return;
        }
    }

    /// find situation where a switch has a state it shouldn't have
    /// and create command to correct that
    pub fn trigger_commands(&mut self) -> Vec<SwitchCommand> {
        let new_room_states = self.get_room_state(Duration::from_secs(0));
        let current_room_states = &self.room_state;

        for (room, new_state) in new_room_states.iter() {
            let old_state = current_room_states.get(room);
            if old_state.is_none() {
                continue;
            }
            let old_state = old_state.unwrap();
            if old_state != new_state {
                trace!(
                    "realized {} changed {} -> {}",
                    room,
                    old_state.to_string(&self.initialisation_time),
                    new_state.to_string(&self.initialisation_time)
                );
            }
        }

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
                    match &new_room_states.get(room).unwrap() {
                        Present => {
                            should_state = Some(On);
                            break 'find_should_state;
                        }
                        AbsentSince(_) => {
                            should_state = Some(Off);
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
        self.room_state = new_room_states;
        commands
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
    fn get_room_state(&self, look_ahead: Duration) -> HashMap<String, SensorMemoryState> {
        let mut rooms = HashMap::new();

        for (room, room_sensors) in self.room_sensors.iter() {
            // current room state contains the state which is interesting for trigger_commands
            // if it is set to AbsentSince() the delay parameter is already considered
            let mut current_room_state: SensorMemoryState = self
                .room_state
                .get(room)
                .filter(|value| value != &&Present)
                .map(|value| value.clone())
                .unwrap_or(Uninitialized);

            'room_state: for (_topic, sensor_memory) in room_sensors.iter() {
                match (&current_room_state, &sensor_memory.state) {
                    (_, Uninitialized) => {}
                    (AbsentSince(current_instant), AbsentSince(new_instant)) => {
                        let new_elapsed = new_instant.elapsed() + look_ahead;
                        if new_elapsed < sensor_memory.delay {
                            current_room_state = Present;
                            break 'room_state;
                        }
                        let current_elapsed = current_instant.elapsed() + look_ahead;
                        if current_elapsed < (new_elapsed - sensor_memory.delay) {
                            continue;
                        }
                        current_room_state =
                            AbsentSince((new_instant.clone() + look_ahead) - sensor_memory.delay);
                    }
                    (Uninitialized, AbsentSince(new_instant)) => {
                        let new_elapsed = new_instant.elapsed() + look_ahead;
                        if new_elapsed < sensor_memory.delay {
                            current_room_state = Present;
                            break 'room_state;
                        }
                        current_room_state =
                            AbsentSince((new_instant.clone() + look_ahead) - sensor_memory.delay);
                    }
                    (_, Present) => {
                        current_room_state = Present;
                        break 'room_state;
                    }
                    (Present, AbsentSince(instant)) => {
                        // this only happens if current_room_state is derived from self.room_state
                        current_room_state = sensor_memory.state.clone();
                    }
                };
            }
            rooms.insert(room.clone(), current_room_state);
        }
        rooms
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

#[derive(PartialEq, Debug, Clone)]
pub enum SensorMemoryState {
    /// Absent since program start
    Uninitialized,
    /// Present
    Present,
    /// was Present once but is now Absent since
    AbsentSince(Instant),
}

impl SensorMemoryState {
    // todo: there should be a nicer version which prints out the date instead seconds
    pub fn to_string(&self, instant: &Instant) -> String {
        match self {
            Uninitialized => {
                return "Uninitialized".to_string();
            }
            Present => {
                return "Present".to_string();
            }
            AbsentSince(since) => {
                let instant_elapsed = instant.elapsed();
                let since_elapsed = since.elapsed();
                let time = if instant_elapsed < since_elapsed {
                    since_elapsed - instant_elapsed
                } else {
                    instant_elapsed - since_elapsed
                };
                return format!("AbsentSince({}s)", time.as_secs());
            }
        }
    }
}

pub struct SwitchMemory {
    pub topic: String,
    pub state: SwitchState,
    pub rooms: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::configuration::{Credentials, Sensor};
    use crate::dummy_configuration::{create_light_switch, create_motion_sensor};
    use std::ops::Sub;
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
        let instant = Instant::now().sub(Duration::from_secs(seconds));
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
                create_sensor(
                    "motion1",
                    vec!["room1".to_string()],
                    10,
                ),
                create_sensor(
                    "motion2",
                    vec!["room1".to_string()],
                    10,
                ),
            ],
            switches: vec![create_light_switch("light1", vec!["room1".to_string()])],
        };
        let mut strategy = Strategy::new(&configuration);

        // test if sensors are proper initialized
        let map = strategy.get_room_state(Duration::from_secs(0));
        assert!(map.get("room1").is_some());
        assert_eq!(
            &SensorMemoryState::Uninitialized,
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

        let instant = instant_from_the_past(12);
        motion_1_sensor.state = AbsentSince(instant);
        let map = strategy.get_room_state(Duration::from_secs(0));
        assert_eq!(
            &SensorMemoryState::AbsentSince(instant - Duration::from_secs(10)),
            map.get("room1").unwrap()
        );
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
        motion_1_sensor.state = AbsentSince(instant);
        let map = strategy.get_room_state(Duration::from_secs(0));
        assert_eq!(&SensorMemoryState::Present, map.get("room1").unwrap());
    }

    #[test]
    fn test_get_room_state_absent_and_delay_with_previous_state() {
        let mut strategy = create_test_setup();
        // setting the room_state to be offline for about 5 secs delay is already included here
        strategy
            .room_state
            .insert("room1".to_string(), AbsentSince(instant_from_the_past(5)));
        let motion_1_sensor = strategy
            .room_sensors
            .get_mut("room1")
            .unwrap()
            .get_mut("motion1");
        assert!(motion_1_sensor.is_some());
        let motion_1_sensor = motion_1_sensor.unwrap();

        let instant = instant_from_the_past(2);
        motion_1_sensor.state = AbsentSince(instant);
        let map = strategy.get_room_state(Duration::from_secs(0));
        assert_eq!(&SensorMemoryState::Present, map.get("room1").unwrap());
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
        motion_1_sensor.state = Present;
        let map = strategy.get_room_state(Duration::from_secs(0));
        assert_eq!(&SensorMemoryState::Present, map.get("room1").unwrap());
    }
}
