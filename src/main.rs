#[macro_use]
extern crate log;
extern crate env_logger;
extern crate paho_mqtt;
extern crate serde_json;

mod configuration;
mod dummy_configuration;
mod mqtt;
mod replay;
mod strategy;

use crate::configuration::{Configuration, SensorState, SwitchState};
use crate::mqtt::MqttClient;
use crate::replay::Replay;
use crate::strategy::{Strategy, SwitchCommand};
use paho_mqtt::MessageBuilder;
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};
use structopt::StructOpt;

const LIGHT_CONTROL_SET_TOPIC: &str = "control/lights/set";
const PING_PERIOD: u64 = 3;

/// commands which can be send to control/lights/set
#[derive(Deserialize)]
pub struct LightControlSetCommand {
    /// change the scene to the given scene name
    pub scene: Option<String>,
}

#[derive(StructOpt)]
#[structopt(name = "basic")]
struct Opt {
    /// Input file (in json)
    #[structopt(name = "config.json", parse(from_os_str))]
    config: PathBuf,
    /// replay script output path
    #[structopt(long, parse(from_os_str))]
    replay_script: Option<PathBuf>,
    /// replay configuration output path
    #[structopt(long, parse(from_os_str))]
    replay_config: Option<PathBuf>,
}

fn main() {
    // //only for development
    //let configuration = hardcoded_config();
    //println!("config: {}", serde_json::to_string(&configuration).unwrap() );
    //std::process::exit(1);

    env_logger::init();
    // parse options
    let opt = Opt::from_args();
    if !opt.config.exists() {
        error!("{}, does not exist", opt.config.to_str().unwrap());
        std::process::exit(1);
    }

    // get configuration
    let configuration = Configuration::load_from_file(&opt.config.to_str().unwrap())
        .expect("couldn't parse configuration");

    let mut replay = None;
    match (opt.replay_config, opt.replay_script) {
        (Some(replay_config), Some(replay_script)) => {
            replay = Some(Replay::new(&replay_script, &replay_config, &configuration).unwrap());
        }
        _ => {}
    }

    // spawn replay thread
    let (replay_sender, replay_receiver): (Sender<ReplayMessage>, Receiver<ReplayMessage>) =
        mpsc::channel();
    let is_replay_enabled = replay.is_some();
    if replay.is_some() {
        let mut replay_tracker = replay.unwrap();
        thread::spawn(move || {
            for message in replay_receiver.iter() {
                replay_tracker.track_message(message.topic.as_str(), message.payload.as_str());
            }
        });
    }

    let mut topics_to_subscribe = configuration.get_topics();
    let mut strategy = Strategy::new(&configuration);
    let light_control_topic = LIGHT_CONTROL_SET_TOPIC.to_string();
    topics_to_subscribe.push(&light_control_topic);
    // connect and subscribe to mqtt
    let mut mqtt_client = MqttClient::new(
        configuration.credentials.host.clone(),
        configuration.credentials.user.clone(),
        configuration.credentials.password.clone(),
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
    let change_sender = update_sender.clone();

    // start thread which reacts on state changes
    let state_configuration = configuration.clone();
    let mqtt_receiver = mqtt_client.cli.start_consuming();
    thread::spawn(move || {
        for msg in mqtt_receiver.iter() {
            if let Some(msg) = msg {
                let topic = msg.topic();
                let payload_str = msg.payload_str();

                if is_replay_enabled {
                    replay_sender.send(ReplayMessage {
                        topic: topic.to_string(),
                        payload: payload_str.to_string(),
                    });
                }

                if topic == LIGHT_CONTROL_SET_TOPIC {
                    let command =
                        serde_json::from_str(&payload_str).map(|a: LightControlSetCommand| a);
                    match command {
                        Err(e) => error!("couldn't parse {} : {}", LIGHT_CONTROL_SET_TOPIC, e),
                        Ok(command) => {
                            command
                                .scene
                                .map(|name| {
                                    state_configuration
                                        .get_scene(&name)
                                        .map(|scene| (name, scene))
                                })
                                .flatten()
                                .map(|(name, scene)| {
                                    info!("change scene to {}", name);
                                    change_sender.send(UpdateMessage::SceneChange(
                                        scene.exclude_switches.clone(),
                                        scene.brightness,
                                        scene.room_tracking_enabled,
                                    ))
                                });
                        }
                    }
                } else {
                    match serde_json::from_str(&payload_str) {
                        Result::Ok(payload) => {
                            state_configuration
                                .get_update_switch_for_topic(topic, &payload)
                                .map(|(topic, state)| {
                                    let content = SwitchChangeContent { topic, state };
                                    change_sender
                                        .send(UpdateMessage::SwitchChange(Instant::now(), content));
                                });
                            state_configuration
                                .get_update_sensor_for_topic(topic, &payload)
                                .map(|(topic, state)| {
                                    let content = SensorChangeContent { topic, state };
                                    change_sender
                                        .send(UpdateMessage::SensorChange(Instant::now(), content));
                                });
                        }
                        _ => {}
                    }
                }
            }
        }
    });

    // start thread that triggers regular ping messages
    let ping_sender = update_sender.clone();
    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(PING_PERIOD));
        ping_sender.send(UpdateMessage::Ping);
    });

    // take over all devices after a while
    let deinit_sender = update_sender.clone();
    let takeover_delay = configuration.get_max_sensor_delay() + 10;
    info!("takeover delay : {}s", takeover_delay);
    thread::spawn(move || {
        let instant = Instant::now();
        thread::sleep(Duration::from_secs(takeover_delay));
        deinit_sender.send(UpdateMessage::Deinit(instant));
    });

    // publish thread
    let publish_configuration = configuration.clone();
    let (publish_sender, publish_receiver): (Sender<SwitchCommand>, Receiver<SwitchCommand>) =
        mpsc::channel();
    thread::spawn(move || {
        for message in publish_receiver.iter() {
            let switch = publish_configuration
                .get_switch_for_topic(message.topic)
                .expect("couldn't get swtich from topic");
            let (topic, command) = switch.get_topic_and_command(message.state, message.brightness);
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
            UpdateMessage::Ping => {}
            UpdateMessage::Deinit(instant) => {
                strategy.replace_uninitialized_with_absents(instant);
            }
            UpdateMessage::SwitchChange(instant, switch_content) => {
                strategy.update_switch(instant, switch_content);
            }
            UpdateMessage::SensorChange(instant, sensor_content) => {
                strategy.update_sensor(instant, sensor_content);
            }
            UpdateMessage::SceneChange(exclude_switches, brightness, room_tracking_enabled) => {
                strategy.set_brightness(brightness);
                strategy.set_room_tracking_enabled(room_tracking_enabled);
                strategy.set_disabled_switches(exclude_switches);
                strategy.trigger_commands();
            }
        };
        strategy.calculate_current_room();
        for switch_command in strategy.trigger_commands() {
            publish_sender.send(switch_command);
        }
    }
}

pub struct ReplayMessage {
    pub topic: String,
    pub payload: String,
}

pub struct PublishMessage {
    pub topic: String,
    pub payload: String,
}

/// Object used to send messages to the main decision engine
pub enum UpdateMessage {
    /// Send a Scene change
    /// * names of excluded topics
    /// * brightness
    /// todo: use named arguments here
    SceneChange(Vec<String>, u8, bool),
    /// Send a State change
    SwitchChange(Instant, SwitchChangeContent),
    /// Send a State change
    SensorChange(Instant, SensorChangeContent),
    /// used to trigger regular calculation
    Ping,
    /// Deinit everything after a while
    Deinit(Instant),
}

pub struct SwitchChangeContent {
    pub topic: String,
    pub state: SwitchState,
}

pub struct SensorChangeContent {
    pub topic: String,
    pub state: SensorState,
}
