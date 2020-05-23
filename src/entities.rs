use std::collections::HashMap;
use std::rc::Rc;

pub struct Home {
    pub name: String,
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
    pub path: Vec<String>,
}

/// A Switch is a device that receives commands
/// like lights on/off
pub struct Switch {
    pub topic: String,
}
