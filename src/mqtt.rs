//extern crate env_logger;
//extern crate log;
//extern crate paho_mqtt;

use futures::future::Future;
use paho_mqtt::{AsyncClient, Message};
use std::rc::Rc;
use std::time::Duration;
use std::{process };

pub struct MqttClient {
    pub cli: AsyncClient,
}

impl MqttClient {
    pub fn new(
        host: String,
        username: String,
        password: String,
        message_callback: fn(&AsyncClient, Option<Message>),
        topics: Vec<Rc<String>>,
    ) -> Self {
        // Create the client. Use an ID for a persistent session.
        // A real system should try harder to use a unique ID.
        let create_opts = paho_mqtt::CreateOptionsBuilder::new()
            .server_uri(host)
            .client_id("rust-io")
            .finalize();

        // Create the client connection
        let mut cli = paho_mqtt::AsyncClient::new(create_opts).unwrap_or_else(|e| {
            println!("Error creating the client: {:?}", e);
            process::exit(1);
        });

        // Attach a closure to the client to receive callback
        // on incoming messages.
        cli.set_message_callback(message_callback);

        // Define the set of options for the connection
        let lwt = paho_mqtt::Message::new("rust-io/light-control", "connection lost", 1);

        let conn_opts = paho_mqtt::ConnectOptionsBuilder::new()
            .keep_alive_interval(Duration::from_secs(20))
            .mqtt_version(paho_mqtt::MQTT_VERSION_3_1_1)
            .clean_session(true)
            .will_message(lwt)
            .password(password)
            .user_name(username)
            .finalize();

        // Make the connection to the broker
        println!("Connecting to the MQTT server...");
        if let Err(err) = cli.connect(conn_opts).wait() {
            eprintln!("Unable to connect: {}", err);
            process::exit(1);
        }

        for topic in topics {
            cli.subscribe(topic.as_str(), 0);
        }

        Self { cli }
    }
}
