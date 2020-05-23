extern crate env_logger;
extern crate log;
extern crate paho_mqtt;

use crate::mqtt::MqttClient;
use std::thread;
use std::time::Duration;

mod configuration;
mod entities;
mod mqtt;

use crate::configuration::hardcoded_config;
use paho_mqtt::{AsyncClient, Message};

fn main() {
    let configuration = hardcoded_config();
    let topics_to_subscribe = configuration.get_topics();

    let test = configuration.clone();

    let callback = move |_cli: &AsyncClient, msg: Option<Message>| {
        if let Some(msg) = msg {
            let topic = msg.topic();
            let payload_str = msg.payload_str();
            println!(
                "{}: {} - {}",
                configuration.topic_to_room.get(topic).unwrap(),
                topic,
                payload_str
            );
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
        // test.print_room_state();
        thread::sleep(Duration::from_secs(10));
    }
}
