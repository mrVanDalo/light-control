use crate::configuration::{Configuration, Credentials, Scene, Sensor, Switch, SwitchCommand};
use std::time::Duration;

/// hard coded for now
pub fn hardcoded_config() -> Configuration {
    let sensors = vec![
        //create_motion_sensor( "test/motion_sensor", rooms: vec!["bed_room".to_string()] ),
        create_motion_sensor("zigbee2mqtt/motion_sensor_2", vec!["bed_room".to_string()]),
        create_motion_sensor("zigbee2mqtt/motion_sensor_7", vec!["bed_room".to_string()]),
        create_motion_sensor(
            "zigbee2mqtt/motion_sensor_1",
            vec!["kitchen_room".to_string()],
        ),
        create_motion_sensor(
            "zigbee2mqtt/motion_sensor_4",
            vec!["living_room".to_string()],
        ),
        create_motion_sensor(
            "zigbee2mqtt/motion_sensor_5",
            vec!["living_room".to_string()],
        ),
        create_motion_sensor("zigbee2mqtt/motion_sensor_5", vec!["bath_room".to_string()]),
        create_motion_sensor("zigbee2mqtt/motion_sensor_8", vec!["bath_room".to_string()]),
        create_door_sensor("zigbee2mqtt/door_sensor_2", vec!["floor_room".to_string()]),
        create_door_sensor("zigbee2mqtt/door_sensor_4", vec!["floor_room".to_string()]),
    ];
    let switches = vec![
        create_light_switch("light_1", vec!["floor_room".to_string()]),
        create_light_switch("light_2", vec!["floor_room".to_string()]),
        create_light_switch("light_3", vec!["living_room".to_string()]),
        create_light_switch("light_4", vec!["bath_room".to_string()]),
        create_light_switch("light_8", vec!["bed_room".to_string()]),
        create_sonoff_switch("PAL01", vec!["bed_room".to_string()]),
        //create_sonoff_switch("PAL02", vec!["bed_room".to_string()]),
        create_sonoff_switch("PAL03", vec!["living_room".to_string()]),
        create_sonoff_switch("PAL04", vec!["bed_room".to_string()]),
        //create_sonoff_switch("PAL05", vec!["living_room".to_string()]),
        create_sonoff_switch("PAL06", vec!["kitchen_room".to_string()]),
    ];
    let scenes = vec![
        Scene {
            room_tracking_enabled: true,
            name: "default".to_string(),
            brightness: 255,
            exclude_switches: vec![],
        },
        Scene {
            room_tracking_enabled: false,
            name: "night".to_string(),
            brightness: 25,
            exclude_switches: vec![
                "stat/PAL01/RESULT".to_string(),
                "stat/PAL03/RESULT".to_string(),
                "stat/PAL04/RESULT".to_string(),
                "zigbee2mqtt/light_2".to_string(),
            ],
        },
    ];

    Configuration {
        credentials: Credentials {
            host: "tcp://pepe.lan:1883".to_string(),
            user: "homeassistant".to_string(),
            password: "hallo".to_string(),
        },
        switches,
        sensors,
        scenes,
    }
}

pub fn create_motion_sensor(topic: &str, rooms: Vec<String>) -> Sensor {
    Sensor {
        topic: topic.to_string(),
        key: "occupancy".to_string(),
        invert_state: false,
        delay: 60,
        rooms,
    }
}
pub fn create_door_sensor(topic: &str, rooms: Vec<String>) -> Sensor {
    Sensor {
        topic: topic.to_string(),
        key: "contact".to_string(),
        invert_state: true,
        delay: 120,
        rooms,
    }
}

pub fn create_light_switch(name: &str, rooms: Vec<String>) -> Switch {
    Switch {
        topic: format!("zigbee2mqtt/{}", name),
        rooms: rooms,
        key: "state".to_string(),
        //state: SwitchState::Off,
        command: SwitchCommand {
            topic: format!("zigbee2mqtt/{}/set", name),
            command: r#"{"state":"{{state}}","brightness":{{brightness}}}"#.to_string(),
            init_command: None,
            on: "ON".to_string(),
            off: "OFF".to_string(),
        },
    }
}

pub fn create_sonoff_switch(name: &str, rooms: Vec<String>) -> Switch {
    Switch {
        topic: format!("stat/{}/RESULT", name),
        rooms: rooms,
        key: "POWER".to_string(),
        //state: SwitchState::Off,
        command: SwitchCommand {
            topic: format!("cmnd/{}/POWER", name),
            command: "{{state}}".to_string(),
            init_command: Some("(null)".to_string()),
            on: "ON".to_string(),
            off: "OFF".to_string(),
        },
    }
}
