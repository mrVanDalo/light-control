use crate::configuration::{Configuration, SensorState, SwitchState};
use crate::{SensorChangeContent, SwitchChangeContent};
use std::collections::HashMap;
use std::time::Instant;

type Topic = String;

pub struct StateMemory {
    pub room_sensors: HashMap<String, HashMap<Topic, SensorMemory>>,
    pub room_switches: Vec<SwitchMemory>,
}

impl StateMemory {
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
                        state: SensorMemoryState::Absent,
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
        StateMemory {
            room_sensors,
            room_switches,
        }
    }

    pub fn update_sensor(&mut self, instant: Instant, sensorContent: SensorChangeContent) {
        for room in self.room_sensors.values_mut() {
            room.get_mut(&sensorContent.topic).map(|sensor_memory| {
                match (&sensor_memory.state, sensorContent.state) {
                    (SensorMemoryState::Absent, SensorState::Absent) => (),
                    (SensorMemoryState::AbsentSince(_), SensorState::Absent) => (),
                    (SensorMemoryState::Absent, SensorState::Present) => {
                        sensor_memory.state = SensorMemoryState::Present
                    }
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

    pub fn update_switch(&mut self, _instant: Instant, switchContent: SwitchChangeContent) {
        for mut room_switch in self.room_switches.iter_mut() {
            if room_switch.topic != switchContent.topic {
                continue;
            }
            room_switch.state = switchContent.state;
            break;
        }
    }

    pub fn trigger_commands(&self) -> Vec<SwitchCommand> {
        use SensorMemoryState::{Absent, AbsentSince, Present};
        // collect current sensor state per room
        let mut rooms = HashMap::new();
        for (room, sensors) in self.room_sensors.iter() {
            let mut room_state = Absent;
            'room_state: for (_topic, state) in sensors.iter() {
                match (&room_state, &state.state) {
                    (_, Present) => {
                        room_state = Present;
                        break 'room_state;
                    }
                    (Present, _) => {
                        break 'room_state;
                    }
                    (_, Absent) => {}
                    (AbsentSince(current_instant), AbsentSince(new_instant)) => {
                        // the youngest instant is needed
                        if current_instant.elapsed() > new_instant.elapsed() {
                            room_state = AbsentSince(new_instant.clone());
                        }
                    }
                    (Absent, AbsentSince(instant)) => {
                        room_state = AbsentSince(instant.clone());
                    }
                };
            }
            rooms.insert(room.clone(), room_state);
        }

        let mut commands = Vec::new();
        for room_sensor in self.room_switches.iter() {
            use SwitchState::{Off, On};
            let mut should_state = Off;
            'find_should_state: for room in room_sensor.rooms.iter() {
                match rooms.get(room).unwrap() {
                    Present => {
                        should_state = On;
                        break 'find_should_state;
                    }
                    _ => {}
                }
            }
            if should_state != room_sensor.state {
                commands.push(SwitchCommand {
                    topic: room_sensor.topic.clone(),
                    state: should_state,
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
    pub state: SensorMemoryState,
}

pub enum SensorMemoryState {
    /// Absent since program start
    Absent,
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
