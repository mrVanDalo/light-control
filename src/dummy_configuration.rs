use crate::configuration::{
    Configuration, Sensor, SensorState, Switch, SwitchCommand, SwitchState,
};

/// hard coded for now
pub fn hardcoded_config() -> Configuration {
    let sensors = vec![
        Sensor {
            topic: "test/motion_sensor".to_string(),
            key: "occupancy".to_string(),
            presents_negator: false,
            state: SensorState::Absent,
            rooms: vec!["bed_room".to_string()],
        },
        Sensor {
            topic: "zigbee2mqtt/motion_sensor_2".to_string(),
            key: "occupancy".to_string(),
            presents_negator: false,
            state: SensorState::Absent,
            rooms: vec!["bed_room".to_string()],
        },
        Sensor {
            topic: "zigbee2mqtt/motion_sensor_7".to_string(),
            key: "occupancy".to_string(),
            presents_negator: false,
            state: SensorState::Absent,
            rooms: vec!["bed_room".to_string()],
        },
        Sensor {
            topic: "zigbee2mqtt/motion_sensor_1".to_string(),
            key: "occupancy".to_string(),
            presents_negator: false,
            state: SensorState::Absent,
            rooms: vec!["kitchen_room".to_string()],
        },
        Sensor {
            topic: "zigbee2mqtt/motion_sensor_4".to_string(),
            key: "occupancy".to_string(),
            presents_negator: false,
            state: SensorState::Absent,
            rooms: vec!["living_room".to_string()],
        },
        Sensor {
            topic: "zigbee2mqtt/motion_sensor_5".to_string(),
            key: "occupancy".to_string(),
            presents_negator: false,
            state: SensorState::Absent,
            rooms: vec!["living_room".to_string()],
        },
        Sensor {
            topic: "zigbee2mqtt/motion_sensor_5".to_string(),
            key: "occupancy".to_string(),
            presents_negator: false,
            state: SensorState::Absent,
            rooms: vec!["bath_room".to_string()],
        },
        Sensor {
            topic: "zigbee2mqtt/motion_sensor_8".to_string(),
            key: "occupancy".to_string(),
            presents_negator: false,
            state: SensorState::Absent,
            rooms: vec!["bath_room".to_string()],
        },
        Sensor {
            topic: "zigbee2mqtt/door_sensor_2".to_string(),
            key: "contact".to_string(),
            presents_negator: true,
            state: SensorState::Absent,
            rooms: vec!["floor_room".to_string()],
        },
        Sensor {
            topic: "zigbee2mqtt/door_sensor_4".to_string(),
            key: "contact".to_string(),
            presents_negator: true,
            state: SensorState::Absent,
            rooms: vec!["floor_room".to_string()],
        },
    ];
    let switches = vec![
        create_light_switch("light_1", vec!["floor_room".to_string()]),
        create_light_switch("light_2", vec!["floor_room".to_string()]),
        create_light_switch("light_3", vec!["living_room".to_string()]),
        create_light_switch("light_4", vec!["bath_room".to_string()]),
        create_light_switch("light_8", vec!["bed_room".to_string()]),
        create_sonoff_switch("PAL01", vec!["bed_room".to_string()]),
        create_sonoff_switch("PAL02", vec!["bed_room".to_string()]),
        create_sonoff_switch("PAL03", vec!["living_room".to_string()]),
        create_sonoff_switch("PAL04", vec!["bed_room".to_string()]),
        create_sonoff_switch("PAL05", vec!["living_room".to_string()]),
        create_sonoff_switch("PAL06", vec!["kitchen_room".to_string()]),
    ];

    Configuration { switches, sensors }
}

fn create_light_switch(name: &str, rooms: Vec<String>) -> Switch {
    Switch {
        topic: format!("zigbee2mqtt/{}", name),
        rooms: rooms,
        key: "state".to_string(),
        state: SwitchState::Off,
        command: SwitchCommand {
            topic: format!("zigbee2mqtt/{}/set", name),
            command: r#"{"state":"{{state}}","brightness":{{brightness}}}"#.to_string(),
            init_command: None,
            on: "ON".to_string(),
            off: "OFF".to_string(),
        },
    }
}

fn create_sonoff_switch(name: &str, rooms: Vec<String>) -> Switch {
    Switch {
        topic: format!("stat/{}/RESULT", name),
        rooms: rooms,
        key: "POWER".to_string(),
        state: SwitchState::Off,
        command: SwitchCommand {
            topic: format!("cmnd/{}/POWER", name),
            command: "{{state}}".to_string(),
            init_command: Some("(null)".to_string()),
            on: "ON".to_string(),
            off: "OFF".to_string(),
        },
    }
}
