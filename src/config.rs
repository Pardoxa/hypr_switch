use serde::{Deserialize, Serialize};

use crate::CONFIG_DIR;

#[derive(Serialize, Deserialize, Debug, Clone, Hash, Default)]
pub struct Config {
    pub monitors_all_replacement: Option<String>,
}

impl Config {
    pub fn parse_from_config_file() -> Self {
        let path = CONFIG_DIR.join("hypr_switch").join("config");

        if let Ok(file) = std::fs::OpenOptions::new().read(true).open(&path) {
            return match serde_json::from_reader(file) {
                Ok(config) => config,
                Err(err) => {
                    panic!("Error while parsing config file: {path:?} - {err:?}")
                }
            };
        }

        let default_config = Config::default();

        // Create ~/.config/hypr_switch
        std::fs::create_dir_all(path.parent().unwrap()).expect("Could not create config directory");

        // Create and write the default config
        let file = std::fs::File::create(&path).expect("Could not create config file");

        serde_json::to_writer_pretty(file, &default_config)
            .expect("Could not write default config");

        default_config
    }
}
