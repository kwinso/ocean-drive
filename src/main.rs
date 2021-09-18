mod auth;
mod config;
mod drive;
mod files;
mod readline;
mod redirect_listener;
extern crate clap;
use clap::{App, SubCommand};

// TODO: Sync local and remote dirs
//  - Create dir in Drive if needed
//  - Create local dir if needed
//  - Sync dirs
//  - Update remote if local is changed
//  - vice versa

#[tokio::main]
async fn main() {
    let matches = App::new("Ocean Drive")
                .version(env!("CARGO_PKG_VERSION"))
                .author(env!("CARGO_PKG_AUTHORS"))
                .about(env!("CARGO_PKG_DESCRIPTION"))
                .subcommand(
                    SubCommand::with_name("auth")
                        .about("Authorization managing.")
                )
                .get_matches();

    let c = files::read_toml::<config::Config>("./config.toml");

    // TODO: Add check for config file in the ~/.config folder. Create if does not exist. Or use the provided one

    if let Some(auth_matches) = matches.subcommand_matches("auth") {
        auth::authorize().await;
    }
}
