extern crate mustache;

use self::mustache::MapBuilder;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::time::Duration;

/// Room setup
#[derive(Clone, Deserialize, Serialize)]
pub struct Configuration {
    pub credentials: Credentials,
    #[serde(default)]
    pub scenes: Vec<Scene>,
    pub sensors: Vec<Sensor>,
    pub switches: Vec<Switch>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Credentials {
    pub host: String,
    pub user: String,
    pub password: String,
}

impl Configuration {
    pub fn load_from_file(path: &str) -> Result<Self, Box<dyn Error>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let configuration = serde_json::from_reader(reader)?;
        return Ok(configuration);
    }

    pub fn get_sensor_for_topic(&self, topic: String) -> Option<&Sensor> {
        for sensor in self.sensors.iter() {
            if sensor.topic == topic {
                return Some(sensor);
            }
        }
        None
    }

    pub fn get_scene(&self, name: &String) -> Option<&Scene> {
        for scene in self.scenes.iter() {
            if &scene.name == name {
                return Some(scene);
            }
        }
        return None;
    }

    pub fn get_switch_for_topic(&self, topic: String) -> Option<&Switch> {
        for switch in self.switches.iter() {
            if switch.topic == topic {
                return Some(switch);
            }
        }
        None
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

    pub fn get_update_sensor_for_topic(
        &self,
        topic: &str,
        payload: &Value,
    ) -> Option<(String, SensorState)> {
        let sensor_state = self
            .get_sensor_for_topic(topic.to_string())
            .map(|sensor| {
                let value = &payload[&sensor.key];
                let presents = SensorState::json_value_to_sensor_state(value);
                let state = if sensor.invert_state {
                    presents.map(|presents| SensorState::negate(presents))
                } else {
                    presents
                };
                state.map(|state| (sensor.topic.clone(), state))
            })
            .flatten();

        sensor_state
    }
    pub fn get_update_switch_for_topic(
        &self,
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
}

/// A Sensor is a device that generates inputs
/// like door open/close or motion detected undetected
///
/// Only mqtt commands that are flat json objects are
/// understood.
#[derive(Clone, Deserialize, Serialize)]
pub struct Sensor {
    /// topic to listen to
    pub topic: String,
    /// json key to read the state
    pub key: String,
    /// rooms that should be considered present when
    /// when this sensor is triggered
    #[serde(default)]
    pub rooms: Vec<String>,
    /// sometimes sensors send false if presents
    /// this options negates presences.
    #[serde(default = "Sensor::default_invert_state")]
    pub invert_state: bool,
    /// delay to wait from present to absent,
    /// when the absent signals appears.
    #[serde(default = "Sensor::default_delay")]
    pub delay: Duration,
}

impl Sensor {
    pub fn default_invert_state() -> bool {
        false
    }
    pub fn default_delay() -> Duration {
        Duration::from_secs(60)
    }
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
#[derive(Clone, Deserialize, Serialize)]
pub struct Switch {
    /// uniq topic to listen for the switch
    pub topic: String,
    /// key for state
    pub key: String,
    /// rooms this switch is placed
    #[serde(default)]
    pub rooms: Vec<String>,
    /// command control
    pub command: SwitchCommand,
}

impl Switch {
    pub fn get_topic_and_command(&self, state: SwitchState, brightness: u8) -> (&String, String) {
        self.command.get_topic_and_command(state, brightness)
    }
}

#[derive(Clone, Deserialize, Serialize)]
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
    #[serde(default)]
    pub init_command: Option<String>,
    /// topic to send the command under
    pub topic: String,
    /// string to send for state argument to run switch on
    #[serde(default = "SwitchCommand::default_on")]
    pub on: String,
    /// string to send for state argument to run switch off
    #[serde(default = "SwitchCommand::default_off")]
    pub off: String,
}

#[derive(Debug, Copy, Clone, PartialEq, Deserialize)]
pub enum SwitchState {
    Unknown,
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
    pub fn default_on() -> String {
        "ON".to_string()
    }
    pub fn default_off() -> String {
        "OFF".to_string()
    }
    pub fn get_topic_and_command(&self, state: SwitchState, brightness: u8) -> (&String, String) {
        debug_assert_ne!(state, SwitchState::Unknown);
        let state_value = match state {
            SwitchState::On => &self.on,
            SwitchState::Off => &self.off,
            SwitchState::Unknown => &self.off, // should never happen
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

    // todo write parse topic tests

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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Scene {
    /// name of the scene
    pub name: String,
    /// brightness level of the scene
    #[serde(default = "Scene::default_brightness")]
    pub brightness: u8,
    /// list all switch topics which should not turned on anymore.
    /// they will be turned off by entering this scene
    #[serde(default)]
    pub exclude_switches: Vec<String>,
}

impl Scene {
    pub fn default_brightness() -> u8 {
        255
    }
}
