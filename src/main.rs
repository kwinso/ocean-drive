mod auth;
mod setup;
mod sync;
mod user;
mod google_drive;
mod files;
mod readline;
mod redirect_listener;
mod parse_url;
extern crate clap;
use clap::{App, SubCommand};
use anyhow::Result;

// TODO: 
//  - Create dir in Drive if needed
//  - Create local dir if needed
//  - Sync dirs
//  - Update remote if local is changed
//  - vice versa
//  - Setup for systemctl
//  - Add icon to tray (idk what would be there, but do it) 
//  - Add functionality to get out of some errors (like with not existing authorization and etc.)
//  - Synced folder can be either the whole drive or folder in the root of the drive

fn main() -> Result<()> {
    let matches = App::new("Ocean Drive")
                .version(env!("CARGO_PKG_VERSION"))
                .author(env!("CARGO_PKG_AUTHORS"))
                .about(env!("CARGO_PKG_DESCRIPTION"))
                .subcommand(
                    SubCommand::with_name("setup")
                        .about("Setup all variables needed start working.")
                )
                .subcommand(
                    SubCommand::with_name("run")
                        .about("Start synchronization.")
                )
                .subcommand(
                    SubCommand::with_name("auth")
                        .about("Run process of app authorization.")
                )
                .get_matches();

    // let c = fjiles::read_toml::<config::Config>("./config.toml");
    // TODO: Add check for config file in the ~/.config folder. Create if does not exist. Or use the provided one from cli args
    
    match matches.subcommand_name() {
        Some("setup") => setup::run(),
        Some("run") => sync::run(),
        Some("auth") => auth::authorize(),
        _ => Ok(())
    }
}
