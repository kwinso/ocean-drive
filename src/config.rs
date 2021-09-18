use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub local_dir: String,
    pub drive: DriveConfig
}

#[derive(Deserialize)]
pub struct DriveConfig {
    pub creds_file: String,
    pub name: String,
    // TODO: Make optional
    pub dir: String
}