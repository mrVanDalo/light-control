use self::mustache::MapBuilder;
use serde_json::Value;
use std::collections::HashSet;

extern crate mustache;

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
    /// topic to listen to
    pub topic: String,
    /// json path to read the state
    pub key: String,
    /// sometimes sensors send false if presents
    /// this options negates presences.
    pub presents_negator: bool,
    /// state to the sensor
    pub state: Presents,
    /// rooms that should be considered present when
    /// when this sensor is triggered
    pub rooms: Vec<String>,
}

/// A Switch is a device that receives commands
/// like lights on/off
pub struct Switch {
    /// uniq topic to listen for the switch
    pub topic: String,
    /// rooms this switch is placed
    pub rooms: Vec<String>,
    /// command control
    pub command: SwitchCommand,
    /// key for state
    pub key: String,
}

impl Switch {
    pub fn get_topic_and_command(&self, state: SwitchState, brightness: u8) -> (&String, String) {
        self.command.get_topic_and_command(state, brightness)
    }
}

pub struct SwitchCommand {
    /// turn on and off command
    /// This is a mustache template. The arguments given are
    /// * state : on/off (see on off statement)
    /// * brightness : 0 - 255
    pub command: String,
    /// topic to send the command under
    pub topic: String,
    /// string to send for state argument to run switch on
    pub on: String,
    /// string to send for state argument to run switch off
    pub off: String,
}

pub enum SwitchState {
    On,
    Off,
}

impl SwitchCommand {
    pub fn get_topic_and_command(&self, state: SwitchState, brightness: u8) -> (&String, String) {
        let state_value = match state {
            SwitchState::On => &self.on,
            SwitchState::Off => &self.off,
        };
        let data = MapBuilder::new()
            .insert("state", state_value)
            .unwrap()
            .insert("brightness", &brightness.to_string())
            .unwrap()
            .build();
        let topic = &self.topic;
        let template = mustache::compile_str(&self.command).expect("couldn't create template ");
        let command = template.render_data_to_string(&data).unwrap();
        (&topic, command)
    }
}

#[cfg(test)]
mod switch_tests {
    use super::*;

    #[test]
    fn test_get_topic_and_command() {
        let switch_command = SwitchCommand {
            command: r#"{"test":{{state}}}"#.to_string(),
            topic: "test/test/test".to_string(),
            on: "1".to_string(),
            off: "0".to_string(),
        };
        let (topic, command) = switch_command.get_topic_and_command(SwitchState::On, 123);
        assert_eq!(topic, "test/test/test");
        assert_eq!(command, r#"{"test":1}"#);
        let (topic, command) = switch_command.get_topic_and_command(SwitchState::Off, 123);
        assert_eq!(topic, "test/test/test");
        assert_eq!(command, r#"{"test":0}"#);
    }

    #[test]
    fn test_get_topic_and_command2() {
        let switch_command = SwitchCommand {
            command: r#"{"state":"{{state}}","brightness":{{brightness}}}"#.to_string(),
            topic: "lights/light_1/set".to_string(),
            on: "ON".to_string(),
            off: "OFF".to_string(),
        };
        let (topic, command) = switch_command.get_topic_and_command(SwitchState::On, 123);
        assert_eq!(topic, "lights/light_1/set");
        assert_eq!(command, r#"{"state":"ON","brightness":123}"#);
        let (topic, command) = switch_command.get_topic_and_command(SwitchState::Off, 123);
        assert_eq!(topic, "lights/light_1/set");
        assert_eq!(command, r#"{"state":"OFF","brightness":123}"#);
    }
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
