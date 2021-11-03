use clap::{App, SubCommand};

pub fn root_subcommand() -> App<'static, 'static> {
    SubCommand::with_name("setup")
        .about("Setups everything needed for app to work.")
        .usage("ocean-drive setup [SUBCOMMAND (if needed)]")
        .after_help("You can use this subcommand without subcommands to setup everthing")
        .subcommand(auth_subcommand())
}

fn auth_subcommand() -> App<'static, 'static> {
    SubCommand::with_name("auth")
        .about("Will get access token from Google API")
        .after_help("It also offers you functionality to automatically create a project in Google Cloud Platform")
}
