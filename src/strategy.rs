use crate::configuration::{Configuration, SensorState, SwitchState};
use crate::strategy::SensorMemoryState::{Present, Initialized, AbsentSince};
use crate::{SensorChangeContent, SwitchChangeContent};
use std::collections::HashMap;
use std::time::{Duration, Instant};

type Topic = String;
type Room = String;
type Sensors = HashMap<Topic, SensorMemory>;

pub struct Strategy {
    /// all known sensors grouped room
    pub room_sensors: HashMap<Room, Sensors>,
    /// all known switches grouped room
    pub room_switches: Vec<SwitchMemory>,

    /// room state cache to print nice messages
    room_state: HashMap<Room, SensorMemoryState>,
    /// room we think the user is located
    pub current_room: Option<Room>,
    /// min_delay of all sensors
    /// this is kinda the buffer of all the sensors
    /// to determine the current_room
    pub min_delay: Duration,
}

impl Strategy {
    /// create a new StateMemory object out of a Configuration
    pub fn new(configuration: &Configuration) -> Self {
        let mut room_sensors = HashMap::new();
        let mut min_delay = Duration::from_secs(300);
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
                        state: SensorMemoryState::Initialized,
                    },
                );
                println!(
                    "{} contains {} with delay: {:?}",
                    room, sensor.topic, sensor.delay
                );
            }
            if sensor.delay < min_delay {
                min_delay = sensor.delay;
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
        if min_delay < Duration::from_secs(10) {
            println!("warning: you have configured a sensor delay below 10 seconds, this can cause wrong location calculation");
        }
        println!("minimal delay: {:?}", min_delay);
        Strategy {
            room_sensors,
            room_switches,
            min_delay,
            room_state: HashMap::new(),
            current_room: None,
        }
    }

    /// after some time none of the sensors can stay on the Initialized state
    pub fn deinit_sensors(&mut self, instant: Instant){
        println!("deinit all");
        for sensor in self.room_sensors.values_mut() {
            for sensor_state in sensor.values_mut() {
                if sensor_state.state == Initialized{
                    sensor_state.state = AbsentSince(instant.clone());
                }
            }
        }
    }

    pub fn update_sensor(&mut self, instant: Instant, sensor_content: SensorChangeContent) {
        for room in self.room_sensors.values_mut() {
            room.get_mut(&sensor_content.topic).map(|sensor_memory| {
                match (&sensor_memory.state, sensor_content.state) {
                    (SensorMemoryState::Initialized, SensorState::Absent) => {
                        sensor_memory.state = SensorMemoryState::AbsentSince(instant);
                    }
                    (SensorMemoryState::Initialized, SensorState::Present) => {
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
        let rooms = self.get_room_state(self.min_delay);

        let delay_play = Duration::from_secs(12);
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
                    present_counter = present_counter +1;
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
                Initialized => {}
            }
        }
        if present_counter > 1 {
            // to much rooms still enabled
            return;
        };
        if present_counter == 1 {
            if self.current_room.is_none(){
                self.current_room = Some(present_room);
                println!(
                    "current_room set to : {}",
                    self.current_room.as_ref().unwrap()
                );
                return;
            }
            if self.current_room.as_ref().unwrap() == &present_room {
                return;
            }
            self.current_room = Some(present_room);
            println!(
                "current_room set to : {}",
                self.current_room.as_ref().unwrap()
            );
            return;
        }
        if youngest_absents + delay_play < current_room_absents {
            self.current_room = Some(youngest_room);
            println!(
                "current_room set to : {}",
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
                println!("turn {}  {:?} -> {:?}", room, old_state.unwrap(), state);
            }
        }

        let mut commands = Vec::new();
        for switch in self.room_switches.iter() {
            use SwitchState::{Off, On};
            let mut should_state = None;
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
            if should_state.is_none() {
                continue;
            }
            if should_state.unwrap() != switch.state {
                println!("turn {:?} -> {}", should_state.unwrap(), switch.topic);
                commands.push(SwitchCommand {
                    topic: switch.topic.clone(),
                    state: should_state.unwrap(),
                })
            }
        }
        // todo : move this on top
        self.room_state = rooms;

        commands
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
                .unwrap_or(Initialized);
            //let mut room_state = Initialized;
            'room_state: for (_topic, state) in sensors.iter() {
                match (&room_state, &state.state) {
                    (Present, _) => {
                        break 'room_state;
                    }
                    (_, Initialized) => {}
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
                    (Initialized, AbsentSince(instant)) => {
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
}

pub struct SensorMemory {
    pub delay: Duration,
    pub state: SensorMemoryState,
}

#[derive(PartialEq, Debug, Clone)]
pub enum SensorMemoryState {
    /// Absent since program start
    Initialized,
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
