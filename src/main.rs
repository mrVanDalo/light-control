extern crate env_logger;
extern crate log;
extern crate paho_mqtt;
extern crate serde_json;

use crate::mqtt::MqttClient;
mod configuration;
mod dummy_configuration;
mod mqtt;
mod state_memory;

use crate::configuration::{Configuration, SensorState, SwitchState};
use crate::dummy_configuration::hardcoded_config;
use crate::state_memory::{StateMemory, SwitchCommand};
use crate::UpdateMessage::SensorChange;
use paho_mqtt::MessageBuilder;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

fn main() {
    let mut configuration = hardcoded_config();
    let topics_to_subscribe = configuration.get_topics();
    let mut state_memory = StateMemory::new(&configuration);

    // connect and subscribe to mqtt
    let mut mqtt_client = MqttClient::new(
        "tcp://pepe.private:1883".to_string(),
        "homeassistant".to_string(),
        "hallo".to_string(),
        topics_to_subscribe,
    );

    // trigger status updates for devices
    for switch in configuration.switches.iter() {
        if switch.command.init_command.is_none() {
            continue;
        }
        let init_command = switch.command.init_command.as_ref().unwrap();
        let message = MessageBuilder::new()
            .topic(&switch.command.topic)
            .payload(init_command.as_str())
            .qos(0)
            .finalize();
        mqtt_client.cli.publish(message);
    }

    // create thread channels
    let (update_sender, update_receiver): (Sender<UpdateMessage>, Receiver<UpdateMessage>) =
        mpsc::channel();
    let ping_sender = update_sender.clone();

    // start thread which reacts on state changes
    let mqtt_receiver = mqtt_client.cli.start_consuming();
    thread::spawn(move || {
        for msg in mqtt_receiver.iter() {
            if let Some(msg) = msg {
                let topic = msg.topic();
                let payload_str = msg.payload_str();

                match serde_json::from_str(&payload_str) {
                    Result::Ok(payload) => {
                        configuration
                            .get_update_switch_for_topic(topic, &payload)
                            .map(|(topic, state)| {
                                let content = SwitchChangeContent { topic, state };
                                update_sender
                                    .send(UpdateMessage::SwitchChange(Instant::now(), content));
                            });

                        configuration
                            .get_update_sensor_for_topic(topic, &payload)
                            .map(|(topic, state)| {
                                let content = SensorChangeContent { topic, state };
                                update_sender
                                    .send(UpdateMessage::SensorChange(Instant::now(), content));
                            });
                    }
                    _ => {}
                }
            }
        }
    });

    // start thread that triggers regular ping messages
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(1000));
        ping_sender.send(UpdateMessage::Ping);
    });

    // publish thread
    let (publish_sender, publish_receiver): (Sender<SwitchCommand>, Receiver<SwitchCommand>) =
        mpsc::channel();
    thread::spawn(move || {
        for message in publish_receiver.iter() {
            let switch = configuration
                .get_switch_for_topic(message.topic)
                .expect("couldn't get swtich from topic");
            let (topic, command) = switch.get_topic_and_command(message.state, 255);
            let mqtt_message = MessageBuilder::new()
                .topic(topic)
                .payload(command)
                .qos(0)
                .finalize();
            mqtt_client.cli.publish(mqtt_message);
        }
    });

    // main loop
    for update_message in update_receiver.iter() {
        match update_message {
            UpdateMessage::Ping => println!("got ping"),
            UpdateMessage::SwitchChange(instant, switch_content) => {
                println!("got state change");
                state_memory.update_switch(instant, switch_content);
            }
            UpdateMessage::SensorChange(instant, sensor_content) => {
                println!("got state change");
                state_memory.update_sensor(instant, sensor_content);
            }
        }
    }
}

pub struct PublishMessage {
    pub topic: String,
    pub payload: String,
}

/// Object used to send messages to the main decision engine
pub enum UpdateMessage {
    /// Send a State change
    SwitchChange(Instant, SwitchChangeContent),
    SensorChange(Instant, SensorChangeContent),
    // used to trigger regular calculation
    Ping,
}

pub struct SwitchChangeContent {
    pub topic: String,
    pub state: SwitchState,
}

pub struct SensorChangeContent {
    pub topic: String,
    pub state: SensorState,
}
