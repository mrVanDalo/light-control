//extern crate env_logger;
//extern crate log;
//extern crate paho_mqtt;

use paho_mqtt::Client;
use std::process;
use std::time::Duration;

pub struct MqttClient {
    pub cli: Client,
}

impl MqttClient {
    pub fn new(host: String, username: String, password: String, topics: Vec<&String>) -> Self {
        // Create the client. Use an ID for a persistent session.
        // A real system should try harder to use a unique ID.
        let create_opts = paho_mqtt::CreateOptionsBuilder::new()
            .server_uri(host)
            .client_id("rust-io")
            .finalize();

        // Create the client connection
        let cli = paho_mqtt::Client::new(create_opts).unwrap_or_else(|e| {
            error!("Error creating the client: {:?}", e);
            process::exit(1);
        });

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
        info!("Connecting to the MQTT server...");
        if let Err(err) = cli.connect(conn_opts) {
            error!("Unable to connect: {}", err);
            process::exit(1);
        }

        for topic in topics {
            cli.subscribe(topic.as_str(), 0).unwrap();
        }

        Self { cli }
    }
}
