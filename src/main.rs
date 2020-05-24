extern crate env_logger;
extern crate log;
extern crate paho_mqtt;
extern crate serde_json;

use crate::mqtt::MqttClient;
mod configuration;
mod dummy_configuration;
mod mqtt;

use crate::dummy_configuration::hardcoded_config;
use serde_json::Value;

fn main() {
    let mut configuration = hardcoded_config();
    let topics_to_subscribe = configuration.get_topics();

    let mut mqtt_client = MqttClient::new(
        "tcp://pepe.private:1883".to_string(),
        "homeassistant".to_string(),
        "hallo".to_string(),
        topics_to_subscribe,
    );

    let receiver = mqtt_client.cli.start_consuming();
    for msg in receiver.iter() {
        if let Some(msg) = msg {
            let topic = msg.topic();
            let payload_str = msg.payload_str();
            let payload: Value = serde_json::from_str(&payload_str).unwrap();

            configuration.update_sensor_for_topic(topic, &payload);
            configuration.print_room_state();
        }
    }
}
