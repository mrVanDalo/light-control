use crate::configuration::{Configuration, Presents, Sensor, Switch, SwitchCommand};

/// hard coded for now
pub fn hardcoded_config() -> Configuration {
    let sensors = vec![
        Sensor {
            topic: "test/motion_sensor".to_string(),
            key: "occupancy".to_string(),
            presents_negator: false,
            state: Presents::Absent,
            rooms: vec!["bed_room".to_string()],
        },
        Sensor {
            topic: "zigbee2mqtt/motion_sensor_2".to_string(),
            key: "occupancy".to_string(),
            presents_negator: false,
            state: Presents::Absent,
            rooms: vec!["bed_room".to_string()],
        },
        Sensor {
            topic: "zigbee2mqtt/motion_sensor_7".to_string(),
            key: "occupancy".to_string(),
            presents_negator: false,
            state: Presents::Absent,
            rooms: vec!["bed_room".to_string()],
        },
        Sensor {
            topic: "zigbee2mqtt/motion_sensor_1".to_string(),
            key: "occupancy".to_string(),
            presents_negator: false,
            state: Presents::Absent,
            rooms: vec!["kitchen_room".to_string()],
        },
        Sensor {
            topic: "zigbee2mqtt/motion_sensor_4".to_string(),
            key: "occupancy".to_string(),
            presents_negator: false,
            state: Presents::Absent,
            rooms: vec!["living_room".to_string()],
        },
        Sensor {
            topic: "zigbee2mqtt/motion_sensor_5".to_string(),
            key: "occupancy".to_string(),
            presents_negator: false,
            state: Presents::Absent,
            rooms: vec!["living_room".to_string()],
        },
        Sensor {
            topic: "zigbee2mqtt/motion_sensor_5".to_string(),
            key: "occupancy".to_string(),
            presents_negator: false,
            state: Presents::Absent,
            rooms: vec!["bath_room".to_string()],
        },
        Sensor {
            topic: "zigbee2mqtt/motion_sensor_8".to_string(),
            key: "occupancy".to_string(),
            presents_negator: false,
            state: Presents::Absent,
            rooms: vec!["bath_room".to_string()],
        },
        Sensor {
            topic: "zigbee2mqtt/door_sensor_2".to_string(),
            key: "contact".to_string(),
            presents_negator: true,
            state: Presents::Absent,
            rooms: vec!["floor_room".to_string()],
        },
        Sensor {
            topic: "zigbee2mqtt/door_sensor_4".to_string(),
            key: "contact".to_string(),
            presents_negator: true,
            state: Presents::Absent,
            rooms: vec!["floor_room".to_string()],
        },
    ];
    let switches = vec![
        create_switch("light_1", vec!["floor_room".to_string()]),
        create_switch("light_2", vec!["floor_room".to_string()]),
        create_switch("light_3", vec!["living_room".to_string()]),
        create_switch("light_4", vec!["bath_room".to_string()]),
        create_switch("light_8", vec!["bed_room".to_string()]),
    ];

    Configuration { switches, sensors }
}

fn create_switch(name: &str, rooms: Vec<String>) -> Switch {
    Switch {
        topic: "zigbee2mqtt/".to_string() + name,
        rooms: rooms,
        key: "state".to_string(),
        command: SwitchCommand {
            topic: ("zigbee2mqtt/".to_string() + name).to_string() + "/set",
            command: r#"{"state":"{{state}}","brightness":{{brightness}}}"#.to_string(),
            on: "ON".to_string(),
            off: "OFF".to_string(),
        },
    }
}
