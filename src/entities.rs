use crate::entities::Presents::Present;
use serde_json::Value;
use std::collections::HashMap;
use std::rc::Rc;

/// The Lights Configuration
pub struct Configuration {
    /// dummy parameter
    pub name: String,

    /// home configuration as it is configured by the user
    pub home: Rc<Home>,

    /// weather or not presents in a room is detected
    /// room -> state
    pub presents: HashMap<String, Presents>,

    /// device -> room
    pub topic_to_room: HashMap<String, String>,

    pub all_sensors: Vec<Rc<Sensor>>,
    pub all_switch: Vec<Rc<Switch>>,
}

impl Configuration {
    pub fn get_topics(&self) -> Vec<Rc<String>> {
        self.home.get_topics()
    }

    pub fn new(home: Rc<Home>) -> Self {
        let mut presents = HashMap::new();
        let mut topic_to_room = HashMap::new();
        let mut all_sensors = Vec::new();
        let mut all_switch = Vec::new();

        let a = home.clone();

        for (name, room) in a.clone().rooms.iter() {
            presents.insert(name.clone(), Presents::Absent);
            for switch in room.switches.iter() {
                topic_to_room.insert(switch.topic.clone(), name.clone());
                all_switch.push(switch.clone());
            }
            for sensor in room.sensors.iter() {
                topic_to_room.insert(sensor.topic.clone(), name.clone());
                all_sensors.push(sensor.clone());
            }
        }

        Self {
            name: "test-setup".to_string(),
            presents,
            topic_to_room,
            all_sensors,
            all_switch,
            home,
        }
    }

    pub fn get_sensor_for_topic(&self, topic: String) -> Option<Rc<Sensor>> {
        for sensor in self.all_sensors.iter() {
            if sensor.topic == topic {
                return Some(sensor.clone());
            }
        }
        return None;
    }

    pub fn get_switch_for_topic(&self, topic: String) -> Option<Rc<Switch>> {
        for switch in self.all_switch.iter() {
            if switch.topic == topic {
                return Some(switch.clone());
            }
        }
        return None;
    }

    // dummy debug function
    pub fn print_room_state(&self) {
        println!("------------------  [ room state ]");
        for (room, presents) in self.presents.iter() {
            println!("{} : {:?}", room, presents);
        }
    }
}

/// A home definition
/// should be entered via JSON
pub struct Home {
    pub rooms: HashMap<String, Rc<Room>>,
}

impl Home {
    pub fn get_topics(&self) -> Vec<Rc<String>> {
        let mut topics = Vec::new();
        for (_name, room) in self.rooms.iter() {
            for topic in room.get_topics() {
                topics.push(topic)
            }
        }
        topics
    }
}

/// Room setup
pub struct Room {
    pub sensors: Vec<Rc<Sensor>>,
    pub switches: Vec<Rc<Switch>>,
}

impl Room {
    pub fn get_topics(&self) -> Vec<Rc<String>> {
        let mut topics = Vec::new();
        for sensor in self.sensors.iter() {
            topics.push(Rc::new(sensor.topic.clone()));
        }
        for switch in self.switches.iter() {
            topics.push(Rc::new(switch.topic.clone()));
        }
        topics
    }
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
}

/// A Switch is a device that receives commands
/// like lights on/off
pub struct Switch {
    pub topic: String,
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
    pub fn json_value_to_presents(value: Value) -> Option<Presents> {
        match value {
            Value::Bool(state) => {
                if state {
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
