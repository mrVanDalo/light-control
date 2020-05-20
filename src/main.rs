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

fn main() {
    let home = hardcoded_config();
    let topics_to_subscribe = home.get_topics();

    let _mqtt_client = MqttClient::new(
        "tcp://pepe.private:1883".to_string(),
        "homeassistant".to_string(),
        "hallo".to_string(),
        |_cli, msg| {
            if let Some(msg) = msg {
                let topic = msg.topic();
                let payload_str = msg.payload_str();
                println!("{} - {}", topic, payload_str);
            }
        },
        topics_to_subscribe,
    );

    // Just wait for incoming messages.
    loop {
        thread::sleep(Duration::from_millis(1000));
    }
}
