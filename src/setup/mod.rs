/* Setup program to be ready to start */

use crate::{auth, files, google_drive::Config as DriveConfig, readline, user};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
mod clap;

pub use self::clap::*;

// TODO: Add function for help
// TODO:    This function should display help message about advanced configuration
// TODO:    in ~/.config/ocean-drive/config.toml file
// TODO: Add configuration for update timeout (how often check for updates from the remote)

#[derive(Deserialize, Serialize, Clone)]
pub struct Config {
    pub local_dir: String,
    pub drive: DriveConfig,
}

pub fn run() -> Result<()> {
    auth::authorize()?;

    create_configuration_dir()?;
    // Todo: some validation for user fields
    set_configurations()?;

    Ok(())
}

/* Creates configuration dir if not exists */
fn create_configuration_dir() -> Result<()> {
    let home = user::get_home()?.join(".config/ocean-drive");

    if !Path::new(&home).exists() {
        fs::create_dir(home)?;
    }
    Ok(())
}

/* Gathers configurations from user and saves it to a file */
fn set_configurations() -> Result<()> {
    let home = user::get_home()?;
    let default_local_dir = &home.join("ocean");

    let local_dir_prompt = "Which directory will be used as local root for your drive?";
    let local_dir = readline::promt_default(local_dir_prompt, default_local_dir.to_str().unwrap());

    let drive_name =
        readline::promt_default("Enter a name for drive that will be synced", "My Drive");
    let remote_dir = readline::promt_default(
        "Enter a name for directory in your drive that will be synced with local directory",
        "ocean",
    );
    println!(
        "\nSaving configuration:\nDirectory '{}' will be up to date with '{}/{}'",
        local_dir, drive_name, remote_dir
    );

    let config = Config {
        local_dir,
        drive: DriveConfig { dir: remote_dir },
    };

    files::write_toml::<Config>(config, home.join(".config/ocean-drive/config.toml"))?;

    Ok(())
}
