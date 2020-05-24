use serde_json::Value;
use std::collections::HashSet;

impl Configuration {
    pub fn update_sensor(&mut self, topic: &str, state: Presents) {
        for sensor in self.sensors.iter_mut() {
            if sensor.topic == topic {
                sensor.state = state;
                println!("set {} -> {:?}", sensor.topic, sensor.state);
                return;
            }
        }
    }

    pub fn get_sensor_for_topic(&self, topic: String) -> Option<&Sensor> {
        for sensor in self.sensors.iter() {
            if sensor.topic == topic {
                return Some(sensor);
            }
        }
        None
    }

    pub fn get_switch_for_topic(&self, topic: String) -> Option<&Switch> {
        for switch in self.switches.iter() {
            if switch.topic == topic {
                return Some(switch);
            }
        }
        None
    }

    // dummy debug function
    pub fn print_room_state(&self) {
        println!("------------------  [ room state ]");
        let rooms = self.rooms();
        for room in rooms {
            let state = self.room_state(&room);
            println!("{} -> {:?}", room, state);
        }
    }

    pub fn room_state(&self, room: &String) -> Presents {
        for sensor in self.sensors.iter() {
            if sensor.rooms.contains(room) {
                if sensor.state == Presents::Present {
                    return Presents::Present;
                }
            }
        }
        Presents::Absent
    }

    pub fn rooms(&self) -> Vec<&String> {
        let mut rooms = HashSet::new();
        for sensor in self.sensors.iter() {
            for room in sensor.rooms.iter() {
                rooms.insert(room);
            }
        }
        let mut result = Vec::new();
        for room in rooms.iter() {
            result.push(*room);
        }
        result
    }

    pub fn get_topics(&self) -> Vec<&String> {
        let mut topics = Vec::new();
        for sensor in self.sensors.iter() {
            topics.push(&sensor.topic);
        }
        for switch in self.switches.iter() {
            topics.push(&switch.topic);
        }
        topics
    }

    pub(crate) fn update_sensor_for_topic(&mut self, topic: &str, payload: &Value) {
        let sensor_presents = self
            .get_sensor_for_topic(topic.to_string())
            .map(|sensor| {
                let value = &payload[&sensor.key];
                let presents = Presents::json_value_to_presents(value);
                if sensor.presents_negator {
                    presents.map(|presents| Presents::negate(presents))
                } else {
                    presents
                }
            })
            .flatten();
        if sensor_presents.is_none() {
            return;
        }

        self.update_sensor(topic, sensor_presents.unwrap());
    }
}

/// Room setup
pub struct Configuration {
    pub sensors: Vec<Sensor>,
    pub switches: Vec<Switch>,
}

/// A Sensor is a device that generates inputs
/// like door open/close or motion detected undetected
pub struct Sensor {
    pub topic: String,
    /// json path to read the state
    pub key: String,

    /// sometimes sensors send false if presents
    /// this options negates presences.
    pub presents_negator: bool,

    pub state: Presents,
    pub rooms: Vec<String>,
}

/// A Switch is a device that receives commands
/// like lights on/off
pub struct Switch {
    pub topic: String,
    pub rooms: Vec<String>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum Presents {
    /// Presents is detected
    Present,
    /// Absents is detected
    Absent,
}

impl Presents {
    pub fn negate(presents: Presents) -> Presents {
        match presents {
            Presents::Absent => Presents::Present,
            Presents::Present => Presents::Absent,
        }
    }
    pub fn json_value_to_presents(value: &Value) -> Option<Presents> {
        match value {
            Value::Bool(state) => {
                if *state {
                    Some(Presents::Present)
                } else {
                    Some(Presents::Absent)
                }
            }
            Value::String(state) => {
                if state.to_ascii_lowercase() == "on" {
                    Some(Presents::Present)
                } else {
                    Some(Presents::Absent)
                }
            }
            _ => None,
        }
    }
}
