use crate::configuration::{Configuration, Presents, Sensor, Switch};

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
        Switch {
            topic: "zigbee2mqtt/light_3".to_string(),
            rooms: vec!["bed_room".to_string()],
        },
        Switch {
            topic: "zigbee2mqtt/light_3".to_string(),
            rooms: vec!["living_room".to_string()],
        },
        Switch {
            topic: "zigbee2mqtt/light_3".to_string(),
            rooms: vec!["bath_room".to_string()],
        },
        Switch {
            topic: "zigbee2mqtt/light_1".to_string(),
            rooms: vec!["floor_room".to_string()],
        },
        Switch {
            topic: "zigbee2mqtt/light_2".to_string(),
            rooms: vec!["floor_room".to_string()],
        },
    ];

    Configuration { switches, sensors }
}
