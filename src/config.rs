use std::io::{Read, Write};

use serde::{Deserialize, Serialize};

use crate::CONFIG_DIR;

#[derive(Serialize, Deserialize, Debug, Clone, Hash, Default)]
pub struct Config {
    pub monitors_all_replacement: Option<String>,
    pub ignore_monitor_type: bool,
}

impl Config {
    pub fn parse_from_config_file() -> Self {
        let path = CONFIG_DIR.join("hypr_switch").join("config.toml");

        match std::fs::OpenOptions::new().read(true).open(&path) {
            Ok(mut file) => {
                let mut buf = String::new();
                let content = match file.read_to_string(&mut buf) {
                    Ok(_) => buf,
                    Err(err) => {
                        panic!("Cannot read content of {path:?} due to {err:?}")
                    }
                };

                return match toml::from_str(&content) {
                    Ok(config) => config,
                    Err(err) => {
                        panic!("Error while parsing config file: {path:?} - {err:?}")
                    }
                };
            }
            Err(err) => {
                println!(
                    "INFO: Could not read file: {err:?}, fallback to default config. Writing default config to file."
                );
            }
        }

        let default_config = Config::default();

        // Create ~/.config/hypr_switch
        std::fs::create_dir_all(path.parent().unwrap()).expect("Could not create config directory");

        // Create and write the default config
        let mut file = std::fs::File::create(&path).expect("Could not create config file");

        let toml_string =
            toml::to_string_pretty(&default_config).expect("Could not create default config");
        file.write_all(toml_string.as_bytes())
            .expect("Should be able to write to file");

        default_config
    }
}
