mod auth;
mod files;
mod google_drive;
mod parse_url;
mod readline;
mod redirect_listener;
mod setup;
mod sync;
mod tray;
mod user;
extern crate clap;
use anyhow::{Result, bail};
use clap::{App, SubCommand};

// TODO:
//  - Create dir in Drive if needed
//  - Create local dir if needed
//  + Sync dirs
//  + Update remote if local is changed
//  - vice versa
//  - Setup for systemctl
//  - Add icon to tray (idk what would be there, but do it)
//  + Add functionality to get out of some errors (like with not existing authorization and etc.)
//  - Synced folder can be either the whole drive or folder in the root of the drive

fn main() -> Result<()> {
    let cmd = App::new("Ocean Drive")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .subcommand(setup::root_subcommand())
        .subcommand(SubCommand::with_name("run").about("[DEFAULT] Start synchronization."))
        .get_matches();

    // let c = files::read_toml::<config::Config>("./config.toml");
    // TODO: Add check for config file in the ~/.config folder. Create if does not exist. Or use the provided one from cli args
    let subcmd = cmd.subcommand_name().unwrap_or("run");

    match subcmd {
        "setup" => setup::run(cmd.subcommand().1.unwrap()),
        "run" => sync::run(),
        _ => {
            bail!("Unknown subcommand. Try 'ocean-drive --help'");
        }
    }
}
