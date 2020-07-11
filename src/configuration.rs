extern crate mustache;

use self::mustache::MapBuilder;
use serde::export::Formatter;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;

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
    pub fn get_max_sensor_delay(&self) -> u64 {
        let mut result = 0;
        for sensor in self.sensors.iter() {
            if result < sensor.delay {
                result = sensor.delay;
            }
        }
        result
    }

    pub fn get_min_sensor_delay(&self) -> u64 {
        let mut result = self.get_max_sensor_delay();
        for sensor in self.sensors.iter() {
            if result > sensor.delay {
                result = sensor.delay;
            }
        }
        result
    }

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
    pub room: String,
    /// sometimes sensors send false if presents
    /// this options negates presences.
    #[serde(default = "Sensor::default_invert_state")]
    pub invert_state: bool,
    /// how long to wait, in seconds, till
    /// a present state becames absent after the devices publishes
    /// the absent message.
    #[serde(default = "Sensor::default_delay")]
    pub delay: u64,
}

impl Sensor {
    pub fn default_invert_state() -> bool {
        false
    }
    pub fn default_delay() -> u64 {
        60
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

    // todo : implement TryFrom<Value> instead of this function
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
    /// how long to wait, in seconds, till the switch is turned off
    /// once it's room becomes the absent state.
    #[serde(default = "Switch::default_delay")]
    pub delay: u64,
}

impl Switch {
    pub fn default_delay() -> u64 {
        0
    }
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
    // todo : implement TryFrom<Value> instead of this function
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
    pub disabled_switches: Vec<String>,
    #[serde(default)]
    pub enabled_switches: Vec<String>,
    #[serde(default)]
    pub ignored_switches: Vec<String>,
    /// tracking enabled or not
    #[serde(default = "Scene::default_room_tracking_enabled")]
    pub room_tracking_enabled: bool,
    /// ignore these sensors
    #[serde(default)]
    pub ignored_sensors: Vec<String>,
}

impl Scene {
    pub fn default_brightness() -> u8 {
        255
    }
    pub fn default_room_tracking_enabled() -> bool {
        true
    }

    /// verify if scene is consistent
    pub fn verify(&self) -> Result<(), Box<dyn Error>> {
        for disabled_switch in self.disabled_switches.iter() {
            if self.enabled_switches.contains(&disabled_switch) {
                error!(
                    "{}, defined as disabled_switch and enabled_switch in {}",
                    disabled_switch, self.name
                );
                return Err(Box::new(ConfigurationError {}));
            }
            if self.ignored_switches.contains(&disabled_switch) {
                error!(
                    "{}, defined as disabled_switch and ignored_switch in {}",
                    disabled_switch, self.name
                );
                return Err(Box::new(ConfigurationError {}));
            }
        }
        for ignored_switch in self.ignored_switches.iter() {
            if self.disabled_switches.contains(&ignored_switch) {
                error!(
                    "{}, defined as ignored_switch and disabled_switch in {}",
                    ignored_switch, self.name
                );
                return Err(Box::new(ConfigurationError {}));
            }
            if self.enabled_switches.contains(&ignored_switch) {
                error!(
                    "{}, defined as ignored_switch and enabled_switch in {}",
                    ignored_switch, self.name
                );
                return Err(Box::new(ConfigurationError {}));
            }
        }
        for enabled_switch in self.enabled_switches.iter() {
            if self.disabled_switches.contains(&enabled_switch) {
                error!(
                    "{}, defined as enabled_switch and disabled_switch in {}",
                    enabled_switch, self.name
                );
                return Err(Box::new(ConfigurationError {}));
            }
            if self.ignored_switches.contains(&enabled_switch) {
                error!(
                    "{}, defined as enabled_switch and ignored_switch in {}",
                    enabled_switch, self.name
                );
                return Err(Box::new(ConfigurationError {}));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod test_scene {
    use super::*;

    #[test]
    fn test_verify1() {
        let scene = Scene {
            name: "".to_string(),
            brightness: 0,
            disabled_switches: vec!["test1".to_string()],
            enabled_switches: vec!["test2".to_string()],
            ignored_switches: vec!["test3".to_string()],
            room_tracking_enabled: false,
            ignored_sensors: vec![],
        };
        match scene.verify() {
            Err(_) => panic!("verification failed but it shouldn't"),
            Ok(_) => {}
        }
    }

    #[test]
    fn test_verify2() {
        let scene = Scene {
            name: "".to_string(),
            brightness: 0,
            disabled_switches: vec!["test1".to_string()],
            enabled_switches: vec!["test1".to_string()],
            ignored_switches: vec!["test3".to_string()],
            room_tracking_enabled: false,
            ignored_sensors: vec![],
        };
        match scene.verify() {
            Ok(_) => panic!("verification successful but it shouldn't"),
            Err(_) => {}
        }
    }

    #[test]
    fn test_verify3() {
        let scene = Scene {
            name: "".to_string(),
            brightness: 0,
            disabled_switches: vec!["test1".to_string()],
            enabled_switches: vec!["test2".to_string()],
            ignored_switches: vec!["test2".to_string()],
            room_tracking_enabled: false,
            ignored_sensors: vec![],
        };
        match scene.verify() {
            Ok(_) => panic!("verification successful but it shouldn't"),
            Err(_) => {}
        }
    }

    #[test]
    fn test_verify4() {
        let scene = Scene {
            name: "".to_string(),
            brightness: 0,
            disabled_switches: vec!["test1".to_string()],
            enabled_switches: vec!["test2".to_string()],
            ignored_switches: vec!["test1".to_string()],
            room_tracking_enabled: false,
            ignored_sensors: vec![],
        };
        match scene.verify() {
            Ok(_) => panic!("verification successful but it shouldn't"),
            Err(_) => {}
        }
    }
}

// todo : create proper Errors and use them everywhere
#[derive(Debug)]
struct ConfigurationError {}

impl Error for ConfigurationError {}
impl std::fmt::Display for ConfigurationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // todo : write proper error message
        write!(f, "not Implemented yet")
    }
}
