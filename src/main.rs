// Mods that contair all functionality for subcommands
mod setup;

mod auth;
mod drive;
mod user;
mod files;
mod readline;
mod redirect_listener;
mod updates;
mod parse_url;
extern crate clap;
use clap::{App, SubCommand, ArgMatches};
use std::process::exit;

// TODO: Sync local and remote dirs
//  - Create dir in Drive if needed
//  - Create local dir if needed
//  - Sync dirs
//  - Update remote if local is changed
//  - vice versa
//  - Setup for systemctl
//  - Add icon to tray (idk what would be there, but do it) 

async fn parse_args<'a>(matches: ArgMatches<'a>) -> Result<(), String> {
    if let Some(_) = matches.subcommand_matches("setup") {
        setup::run().await?;
    }
    if let Some(_) = matches.subcommand_matches("run") {
        updates::Updates::new().watch().await?;
    }

    Ok(())
}

#[tokio::main]
async fn main() {
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
                .get_matches();

    // let c = files::read_toml::<config::Config>("./config.toml");

    // TODO: Add check for config file in the ~/.config folder. Create if does not exist. Or use the provided one
    

    if let Err(e) = parse_args(matches).await {
        eprintln!("{}", e);
        exit(1);
    }
}
