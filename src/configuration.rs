use crate::entities::{Home, Room, Sensor, Switch};
use std::collections::HashMap;
use std::rc::Rc;

/// hard coded for now
pub fn hardcoded_config() -> Rc<Home> {
    let mut rooms = HashMap::new();
    rooms.insert(
        "living_room".to_string(),
        Rc::new(Room {
            sensors: vec![Rc::new(Sensor {
                topic: "zigbee2mqtt/motion_sensor_2".to_string(),
                path: vec!["occupancy".to_string()],
            })],
            switches: vec![Rc::new(Switch {
                topic: "zigbee2mqtt/light_4".to_string(),
            })],
        }),
    );
    Rc::new(Home { rooms: rooms })
}
