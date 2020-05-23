extern crate env_logger;
extern crate log;
extern crate paho_mqtt;
extern crate serde_json;

use crate::mqtt::MqttClient;
use std::thread;
use std::time::Duration;

mod configuration;
mod entities;
mod mqtt;

use crate::configuration::hardcoded_config;
use crate::entities::Presents;
use paho_mqtt::{AsyncClient, Message};
use serde_json::Value;

fn main() {
    let configuration = hardcoded_config();
    let topics_to_subscribe = configuration.get_topics();

    let callback = move |_cli: &AsyncClient, msg: Option<Message>| {
        if let Some(msg) = msg {
            let topic = msg.topic();
            let payload_str = msg.payload_str();
            let payload: Value = serde_json::from_str(&payload_str).unwrap();
            let sensor_presents = configuration
                .get_sensor_for_topic(topic.to_string())
                .map(|sensor| {
                    let value = payload[&sensor.key].clone();
                    let presents = Presents::json_value_to_presents(value);
                    if sensor.presents_negator {
                        presents.map(|presents| Presents::negate(presents))
                    } else {
                        presents
                    }
                })
                .flatten();

            match sensor_presents {
                Some(state) => {
                    println!(
                        "detected {}: {} -> {:?}",
                        configuration.topic_to_room.get(topic).unwrap(),
                        topic,
                        state
                    );
                }
                _ => {
                    println!(
                        "{}: {} - {}",
                        configuration.topic_to_room.get(topic).unwrap(),
                        topic,
                        payload_str
                    );
                }
            }
        }
    };

    let _mqtt_client = MqttClient::new(
        "tcp://pepe.private:1883".to_string(),
        "homeassistant".to_string(),
        "hallo".to_string(),
        Box::new(callback),
        topics_to_subscribe,
    );

    // Just wait for incoming messages.
    loop {
        thread::sleep(Duration::from_secs(10));
    }
}
