use crate::configuration::{Configuration, SensorState, SwitchState};
use crate::strategy::SensorMemoryState::{AbsentSince, Present, Uninitialized};
use crate::{SensorChangeContent, SwitchChangeContent};
use std::collections::HashMap;
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
    room_state: HashMap<Room, SensorMemoryState>,

    /// room we think the user is located
    current_room: Option<Room>,

    /// min_delay of all sensors
    /// this is kinda the buffer of all the sensors
    /// to determine the current_room
    look_ahead_delay: Duration,

    /// switch topics which should be permanent disabled
    disabled_switches: Vec<String>,

    /// current brightness
    brightness: u8,
}

impl Strategy {
    /// create a new StateMemory object out of a Configuration
    pub fn new(configuration: &Configuration) -> Self {
        let mut room_sensors = HashMap::new();
        let mut look_ahead_delay = Duration::from_secs(300);
        for sensor in configuration.sensors.iter() {
            for room in sensor.rooms.iter() {
                if !room_sensors.contains_key(room) {
                    room_sensors.insert(room.clone(), HashMap::new());
                }
                let sensors_memory = room_sensors.get_mut(room).unwrap();
                // initial everything is Absent
                sensors_memory.insert(
                    sensor.topic.clone(),
                    SensorMemory {
                        delay: sensor.delay,
                        state: SensorMemoryState::Uninitialized,
                    },
                );
                info!(
                    "{} contains {} with delay: {:?}",
                    room, sensor.topic, sensor.delay
                );
            }
            if sensor.delay < look_ahead_delay {
                look_ahead_delay = sensor.delay;
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
        if look_ahead_delay < Duration::from_secs(10) {
            warn!("warning: you have configured a sensor delay below 10 seconds, this can cause wrong location calculation");
        }
        info!("look ahead delay: {:?}", look_ahead_delay);

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
            room_sensors,
            room_switches,
            look_ahead_delay,
            room_state: HashMap::new(),
            current_room: None,
            disabled_switches,
            brightness,
        }
    }

    /// after some time none of the sensors can stay on the Initialized state
    pub fn replace_uninitialized_with_absents(&mut self, instant: Instant) {
        info!("take over uninitialized sensors, previous state will now be set to expected state for all controlled devices");
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

    // todo : das ist noch buggy
    pub fn calculate_current_room(&mut self) {
        let rooms = self.get_room_state(self.look_ahead_delay);

        // The play time the instant of the next location has to be younger
        // than the current location
        let delay_play = self.look_ahead_delay / 2;

        let mut current_room_absents = Duration::from_secs(60 * 55);
        let mut youngest_absents = Duration::from_secs(60 * 60);
        debug_assert!(youngest_absents + delay_play > current_room_absents);
        let mut youngest_room = "".to_string();
        let mut present_counter = 0;
        let mut present_room = "".to_string();
        for (room, state) in rooms {
            match state {
                Present => {
                    // if one of the rooms is still present, we don't need to calculate anything
                    //return;
                    present_counter = present_counter + 1;
                    present_room = room;
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
        if present_counter > 1 {
            // to much rooms still enabled
            return;
        };

        if present_counter == 1 {
            if self.current_room.is_none() {
                self.current_room = Some(present_room);
                debug!(
                    "because of single presents , current_room is set to : {}",
                    self.current_room.as_ref().unwrap()
                );
                return;
            }
            if self.current_room.as_ref().unwrap() == &present_room {
                return;
            }
            // current_room_absents needs to be set now
            if current_room_absents < delay_play {
                return;
            }
            // if only one room present and the current room is not present
            // for delay_play set the present_room to current_room
            self.current_room = Some(present_room);
            debug!(
                "because current_room is to long absent ({:?}), new current_room is set to : {}",
                current_room_absents,
                self.current_room.as_ref().unwrap()
            );
            return;
        }

        if youngest_absents + delay_play < current_room_absents {
            self.current_room = Some(youngest_room);
            debug!(
                "because of current_room ({:?} is longer absent than another room ({:?} + {:?} play), current_room is set to : {}",
                current_room_absents,
                youngest_absents,
                delay_play,
                self.current_room.as_ref().unwrap()
            );
            return;
        }
    }

    /// find situation where a switch has a state which it shouldn't have
    /// and create commands to change that
    pub fn trigger_commands(&mut self) -> Vec<SwitchCommand> {
        let rooms = self.get_room_state(Duration::from_secs(0));
        for (room, state) in rooms.iter() {
            let old_state = self.room_state.get(room);
            if old_state.is_none() {
                continue;
            }
            if old_state.as_ref().unwrap() != &state {
                trace!("turn {}  {:?} -> {:?}", room, old_state.unwrap(), state);
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
                    if Some(room) == self.current_room.as_ref() {
                        should_state = Some(On);
                        break 'find_should_state;
                    }
                    match &rooms.get(room).unwrap() {
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
                trace!("turn {:?} -> {}", should_state.unwrap(), switch.topic);
                commands.push(SwitchCommand {
                    topic: switch.topic.clone(),
                    state: should_state.unwrap(),
                    brightness: self.brightness,
                })
            }
        }
        // todo : move this on top
        self.room_state = rooms;

        commands
    }

    pub fn update_brightness(&mut self, brightness: u8) {
        self.brightness = brightness;
    }

    pub fn update_disabled_switches(&mut self, disabled_switches: Vec<String>) {
        self.disabled_switches = disabled_switches;
    }

    /// the current state of the room.
    /// sensor delays are taken into account
    ///
    /// # Arguments
    ///
    /// * `delay_buffer` - shorten the delay of all sensors, to get headroom for calculations
    ///
    fn get_room_state(&self, delay_buffer: Duration) -> HashMap<String, SensorMemoryState> {
        let mut rooms = HashMap::new();
        for (room, sensors) in self.room_sensors.iter() {
            let mut room_state: SensorMemoryState = self
                .room_state
                .get(room)
                .filter(|value| value != &&Present)
                .map(|value| value.clone())
                .unwrap_or(Uninitialized);
            //let mut room_state = Initialized;
            'room_state: for (_topic, state) in sensors.iter() {
                match (&room_state, &state.state) {
                    (Present, _) => {
                        break 'room_state;
                    }
                    (_, Uninitialized) => {}
                    (AbsentSince(current_instant), AbsentSince(new_instant)) => {
                        if (new_instant.elapsed() + delay_buffer) < state.delay {
                            continue;
                        }
                        if (current_instant.elapsed() + delay_buffer)
                            < (new_instant.elapsed() + delay_buffer) - state.delay
                        {
                            continue;
                        }
                        room_state =
                            AbsentSince((new_instant.clone() + delay_buffer) - state.delay);
                    }
                    (Uninitialized, AbsentSince(instant)) => {
                        if (instant.elapsed() + delay_buffer) < state.delay {
                            room_state = Present;
                            continue;
                        }
                        room_state = AbsentSince((instant.clone() + delay_buffer) - state.delay);
                    }
                    (_, Present) => {
                        room_state = Present;
                        break 'room_state;
                    }
                };
            }
            rooms.insert(room.clone(), room_state);
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

pub struct SwitchMemory {
    pub topic: String,
    pub state: SwitchState,
    pub rooms: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
