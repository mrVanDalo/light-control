extern crate mustache;

use self::mustache::MapBuilder;
use serde_json::Value;
use std::collections::HashSet;

/// Room setup
#[derive(Clone)]
pub struct Configuration {
    pub sensors: Vec<Sensor>,
    pub switches: Vec<Switch>,
}

impl Configuration {
    pub fn update_switch(&mut self, topic: &str, state: SwitchState) {
        for switch in self.switches.iter_mut() {
            if switch.topic == topic {
                switch.state = state;
                println!("set {} -> {:?}", switch.topic, switch.state);
                return;
            }
        }
    }

    pub fn update_sensor(&mut self, topic: &str, state: SensorState) {
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
        let rooms = self.get_rooms();
        println!("");
        println!("");
        for room in rooms {
            let state = self.get_room_sensor_state(&room);
            println!("");
            println!("{} ({:?})", room, state);
            println!("--------------------");
            for sensor in self.get_sensors_for_room(room) {
                println!(" {:<30}: {:?}", sensor.topic, sensor.state);
            }
            for switch in self.get_switches_for_room(room) {
                println!(" {:<30}: {:?}", switch.topic, switch.state);
            }
        }
    }

    pub fn get_sensors_for_room(&self, room: &String) -> Vec<&Sensor> {
        let mut result = Vec::new();
        for sensor in self.sensors.iter() {
            if sensor.rooms.contains(room) {
                result.push(sensor);
            }
        }
        result
    }

    pub fn get_switches_for_room(&self, room: &String) -> Vec<&Switch> {
        let mut result = Vec::new();
        for switch in self.switches.iter() {
            if switch.rooms.contains(room) {
                result.push(switch);
            }
        }
        result
    }

    pub fn get_room_sensor_state(&self, room: &String) -> SensorState {
        for sensor in self.sensors.iter() {
            if sensor.rooms.contains(room) {
                if sensor.state == SensorState::Present {
                    return SensorState::Present;
                }
            }
        }
        SensorState::Absent
    }

    pub fn get_rooms(&self) -> Vec<&String> {
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
        result.sort();
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

    pub(crate) fn get_update_sensor_for_topic(
        &mut self,
        topic: &str,
        payload: &Value,
    ) -> Option<(String, SensorState)> {
        let sensor_state = self
            .get_sensor_for_topic(topic.to_string())
            .map(|sensor| {
                let value = &payload[&sensor.key];
                let presents = SensorState::json_value_to_sensor_state(value);
                let state = if sensor.presents_negator {
                    presents.map(|presents| SensorState::negate(presents))
                } else {
                    presents
                };
                state.map(|state| (sensor.topic.clone(), state))
            })
            .flatten();

        sensor_state
    }
    pub(crate) fn get_update_switch_for_topic(
        &mut self,
        topic: &str,
        payload: &Value,
    ) -> Option<(String, SwitchState)> {
        let switch_state = self
            .get_switch_for_topic(topic.to_string())
            .map(|switch| {
                let value = &payload[&switch.key];
                let state = SwitchState::json_value_to_switch_state(value);
                state.map(|state| (switch.topic.clone(), state))
            })
            .flatten();

        switch_state
    }

    pub(crate) fn update_sensor_for_topic(&mut self, topic: &str, payload: &Value) {
        let sensor_presents = self
            .get_sensor_for_topic(topic.to_string())
            .map(|sensor| {
                let value = &payload[&sensor.key];
                let presents = SensorState::json_value_to_sensor_state(value);
                if sensor.presents_negator {
                    presents.map(|presents| SensorState::negate(presents))
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

    pub fn update_switch_for_topic(&mut self, topic: &str, payload: &Value) {
        let switch_presents = self
            .get_switch_for_topic(topic.to_string())
            .map(|switch| {
                let value = &payload[&switch.key];
                let presents = SwitchState::json_value_to_switch_state(value);
                presents
            })
            .flatten();
        if switch_presents.is_none() {
            return;
        }

        self.update_switch(topic, switch_presents.unwrap());
    }
}

/// A Sensor is a device that generates inputs
/// like door open/close or motion detected undetected
#[derive(Clone)]
pub struct Sensor {
    /// topic to listen to
    pub topic: String,
    /// json path to read the state
    pub key: String,
    /// rooms that should be considered present when
    /// when this sensor is triggered
    pub rooms: Vec<String>,
    /// state to the sensor
    pub state: SensorState,
    /// sometimes sensors send false if presents
    /// this options negates presences.
    pub presents_negator: bool,
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum SensorState {
    /// Presents is detected
    Present,
    /// Absents is detected
    Absent,
}

impl SensorState {
    pub fn negate(presents: SensorState) -> SensorState {
        match presents {
            SensorState::Absent => SensorState::Present,
            SensorState::Present => SensorState::Absent,
        }
    }

    pub fn json_value_to_sensor_state(value: &Value) -> Option<SensorState> {
        use SensorState::{Absent, Present};
        match value {
            Value::Bool(state) => {
                if *state {
                    Some(Present)
                } else {
                    Some(Absent)
                }
            }
            Value::String(state) => {
                if state.to_ascii_lowercase() == "on" {
                    Some(Present)
                } else {
                    Some(Absent)
                }
            }
            _ => None,
        }
    }
}

/// A Switch is a device that receives commands
/// like lights on/off
#[derive(Clone)]
pub struct Switch {
    /// uniq topic to listen for the switch
    pub topic: String,
    /// key for state
    pub key: String,
    /// rooms this switch is placed
    pub rooms: Vec<String>,
    /// state of the switch
    pub state: SwitchState,
    /// command control
    pub command: SwitchCommand,
}

impl Switch {
    pub fn get_topic_and_command(&self, state: SwitchState, brightness: u8) -> (&String, String) {
        self.command.get_topic_and_command(state, brightness)
    }
}

#[derive(Clone)]
pub struct SwitchCommand {
    /// turn on and off command
    /// This is a mustache template. The arguments given are
    /// * state : on/off (see on off statement)
    /// * brightness : 0 - 255
    /// * rgb (todo)
    /// * warmth (todo)
    pub command: String,
    /// command to get state of the device
    /// useful at program start.
    pub init_command: Option<String>,
    /// topic to send the command under
    pub topic: String,
    /// string to send for state argument to run switch on
    pub on: String,
    /// string to send for state argument to run switch off
    pub off: String,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SwitchState {
    On,
    Off,
}

impl SwitchState {
    pub fn json_value_to_switch_state(value: &Value) -> Option<SwitchState> {
        use SwitchState::{Off, On};
        match value {
            Value::Bool(state) => {
                if *state {
                    Some(On)
                } else {
                    Some(Off)
                }
            }
            Value::String(state) => {
                if state.to_ascii_lowercase() == "on" {
                    Some(On)
                } else {
                    Some(Off)
                }
            }
            _ => None,
        }
    }
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
            init_command: None,
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
            init_command: None,
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
