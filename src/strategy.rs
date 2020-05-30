use crate::configuration::{Configuration, SensorState, SwitchState};
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
    /// room we think the user is located
    pub current_room: Option<Room>,
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
                // initial everything is Absent
                sensors_memory.insert(
                    sensor.topic.clone(),
                    SensorMemory {
                        delay: sensor.delay,
                        state: SensorMemoryState::Initialized,
                    },
                );
            }
        }
        let mut room_switches = Vec::new();
        for switch in configuration.switches.iter() {
            room_switches.push(SwitchMemory {
                topic: switch.topic.clone(),
                state: SwitchState::Off,
                rooms: switch.rooms.clone(),
            })
        }
        Strategy {
            room_sensors,
            room_switches,
            current_room: None,
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

    /// find situation where a switch has a state which it shouldn't have
    /// and create commands to change that
    pub fn trigger_commands(&self) -> Vec<SwitchCommand> {
        use SensorMemoryState::{AbsentSince, Initialized, Present};
        // collect current sensor state per room
        let mut rooms = HashMap::new();
        for (room, sensors) in self.room_sensors.iter() {
            let mut room_state = Initialized;
            'room_state: for (_topic, state) in sensors.iter() {
                match (&room_state, &state.state) {
                    (Present, _) => {
                        break 'room_state;
                    }
                    (_, Initialized) => {}
                    (AbsentSince(current_instant), AbsentSince(new_instant)) => {
                        if new_instant.elapsed() < state.delay {
                            continue;
                        }
                        if current_instant.elapsed() < new_instant.elapsed() - state.delay {
                            continue;
                        }
                        room_state = AbsentSince(new_instant.clone() - state.delay);
                    }
                    (Initialized, AbsentSince(instant)) => {
                        if instant.elapsed() < state.delay {
                            continue;
                        }
                        room_state = AbsentSince(instant.clone() - state.delay);
                    }
                    (_, Present) => {
                        room_state = Present;
                        break 'room_state;
                    }
                };
            }
            rooms.insert(room.clone(), room_state);
        }

        let mut commands = Vec::new();
        for switch in self.room_switches.iter() {
            use SwitchState::{Off, On};
            let mut should_state = None;
            'find_should_state: for room in switch.rooms.iter() {
                match rooms.get(room).unwrap() {
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
                commands.push(SwitchCommand {
                    topic: switch.topic.clone(),
                    state: should_state.unwrap(),
                })
            }
        }
        commands
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
