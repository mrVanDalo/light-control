use crate::entities::{Configuration, Home, Room, Sensor, Switch};
use std::collections::HashMap;
use std::rc::Rc;

/// hard coded for now
pub fn hardcoded_config() -> Rc<Configuration> {
    let mut rooms = HashMap::new();
    rooms.insert(
        "bed_room".to_string(),
        Rc::new(Room {
            sensors: vec![
                Rc::new(Sensor {
                    topic: "zigbee2mqtt/motion_sensor_2".to_string(),
                    path: vec!["occupancy".to_string()],
                }),
                Rc::new(Sensor {
                    topic: "zigbee2mqtt/motion_sensor_7".to_string(),
                    path: vec!["occupancy".to_string()],
                }),
            ],
            switches: vec![Rc::new(Switch {
                topic: "zigbee2mqtt/light_3".to_string(),
            })],
        }),
    );
    rooms.insert(
        "kitchen_room".to_string(),
        Rc::new(Room {
            sensors: vec![Rc::new(Sensor {
                topic: "zigbee2mqtt/motion_sensor_1".to_string(),
                path: vec!["occupancy".to_string()],
            })],
            switches: vec![],
        }),
    );
    rooms.insert(
        "living_room".to_string(),
        Rc::new(Room {
            sensors: vec![
                Rc::new(Sensor {
                    topic: "zigbee2mqtt/motion_sensor_4".to_string(),
                    path: vec!["occupancy".to_string()],
                }),
                Rc::new(Sensor {
                    topic: "zigbee2mqtt/motion_sensor_5".to_string(),
                    path: vec!["occupancy".to_string()],
                }),
            ],
            switches: vec![Rc::new(Switch {
                topic: "zigbee2mqtt/light_3".to_string(),
            })],
        }),
    );
    rooms.insert(
        "bath_room".to_string(),
        Rc::new(Room {
            sensors: vec![
                Rc::new(Sensor {
                    topic: "zigbee2mqtt/motion_sensor_5".to_string(),
                    path: vec!["occupancy".to_string()],
                }),
                Rc::new(Sensor {
                    topic: "zigbee2mqtt/motion_sensor_8".to_string(),
                    path: vec!["occupancy".to_string()],
                }),
            ],
            switches: vec![Rc::new(Switch {
                topic: "zigbee2mqtt/light_3".to_string(),
            })],
        }),
    );

    rooms.insert(
        "floor_room".to_string(),
        Rc::new(Room {
            sensors: vec![
                Rc::new(Sensor {
                    topic: "zigbee2mqtt/door_sensor_2".to_string(),
                    path: vec!["occupancy".to_string()],
                }),
                Rc::new(Sensor {
                    topic: "zigbee2mqtt/door_sensor_4".to_string(),
                    path: vec!["occupancy".to_string()],
                }),
            ],
            switches: vec![
                Rc::new(Switch {
                    topic: "zigbee2mqtt/light_1".to_string(),
                }),
                Rc::new(Switch {
                    topic: "zigbee2mqtt/light_2".to_string(),
                }),
            ],
        }),
    );

    Rc::new(Configuration::new(Rc::new(Home { rooms })))
}
