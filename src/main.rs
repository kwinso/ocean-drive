mod log;
mod updates;
mod drive;
use std::fs;
use std::path::{PathBuf};
use toml;
use serde::Deserialize;
use drive::GoogleDrive;

#[derive(Deserialize)]
struct Config {
    local_dir: String
}

fn load_config(path: &str) -> Config  {
    let absolute_path = match fs::canonicalize(&PathBuf::from(path)) {
        Ok(path) => path,
        Err(_) => {
            log::error(format!("No config file in {:?}", path));
            std::process::exit(1);
        }
    };

    let contents = fs::read_to_string(absolute_path).unwrap();
    match toml::from_str(&contents) {
        Err(e) => { 
            log::error(format!("Unable to load the config: {}", e));
            std::process::exit(1);
        }
        Ok(r) => r
    }
}

fn main() {
    let config_path = "./config.toml";

    log::info(format!("Loading config file from {:?}", config_path));
    let config = load_config(config_path);

    log::info(format!("Root directory detected: {:?}", config.local_dir));

    let drive = GoogleDrive::new();
}