//! to create replay scripts that can be used for integration tests
//! or debugging false behavior

use crate::configuration::Configuration;
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

pub struct Replay {
    replay_script_output: File,
    instant: Instant,
}

impl Replay {
    pub fn new(
        replay_script_output: &PathBuf,
        configuration_output: &PathBuf,
        configuration: &Configuration,
    ) -> Result<Self, Box<dyn Error>> {
        let mut configuration_file = File::create(configuration_output)?;
        serde_json::to_writer_pretty(configuration_file, configuration).unwrap();
        let mut file = File::create(replay_script_output)?;
        write!(file, "#!/usr/bin/env bash\n");
        write!(
            file,
            "# replay_script for configuration {}\n",
            configuration_output.to_str().unwrap()
        );
        write!(
            file,
            r#"
function sleep_for_some_seconds(){{
  sleep($1)
}}

function publish(){{
  # host is usually without port and scheme
  mosquitto_pub -h {} -u {} -P {} -t "$1" -m "$2"
}}

"#,
            configuration.credentials.host, // todo: not really correct
            configuration.credentials.user,
            configuration.credentials.password
        );
        Ok(Replay {
            replay_script_output: file,
            instant: Instant::now(),
        })
    }

    pub fn track_message(&mut self, topic: &str, payload: &str) {
        write!(
            self.replay_script_output,
            "sleep_for_some_seconds({})\n",
            self.instant.elapsed().as_secs()
        );
        write!(
            self.replay_script_output,
            "publish '{}' '{}'\n",
            topic, payload
        );
        self.instant = Instant::now();
    }
}
