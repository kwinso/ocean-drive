mod remote;
mod versions;
use crate::{
    auth::Creds,
    files,
    google_drive::{Client, Session},
    setup::Config as AppConfig,
    user,
};
use anyhow::{bail, Result};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use versions::Versions;
/*
    Setups two daemons for updates: local and remote.
    Each of them is responsible for either downloading files from the remote, or uploading local files to the remote
    Each of daemons will be in the own thread.
    Threads will share a mutable referce to drive client, this will allow to keep the same authroziation
    while app is running.
*/
pub fn run() -> Result<()> {
    let conf_dir = user::get_home()?.join(".config/ocean-drive");
    let conf_file = conf_dir.join("config.toml");

    let client = Arc::new(Mutex::new(setup_client(&conf_dir)?));
    let config = files::read_toml::<AppConfig>(conf_file)?;
    let versions = Arc::new(Mutex::new(Versions::new(conf_dir.join("versions.json"))?));

    let c = Arc::clone(&client);
    let ver = Arc::clone(&versions);
    let remote_watcher = thread::spawn(move || {
        let mut remote_manager = remote::RemoteDaemon::new(config, c.clone(), ver).unwrap();
        remote_manager.start().unwrap();
    });

    let c = Arc::clone(&client);
    // let local_watcher = thread::spawn(move || {});

    remote_watcher.join().unwrap();

    Ok(())
}

fn setup_client(conf_dir: &PathBuf) -> Result<Client> {
    let session_file = conf_dir.join("session.toml");
    let creds_file = conf_dir.join("creds.toml");

    let session;
    let creds = files::read_toml::<Creds>(creds_file)?;

    match files::read_toml::<Session>(session_file.clone()) {
        Ok(s) => {
            session = s;
        }
        Err(_) => bail!("Unable to read access authorization data.\nTip: Try to run `ocean-drive auth` to update authorization data"),
    };

    let mut client = Client::new(
        creds.client_id.clone(),
        creds.client_secret.clone(),
        "https://localhost:8080".to_string(),
    );

    client.set_session(session.clone());

    if session.refresh_token.is_some() {
        match client.refresh_token() {
            Ok(new_session) => {
                files::write_toml(new_session, session_file)?;

                println!("Info: Authorization for client is updated.");
            }
            Err(_) => eprintln!("Warn: App was unable to update Google API Access Token.\nTip: Try to manually authorize using `ocean-drive auth`."),
        };
    } else {
        println!("Warn: No refresh token for client is provided!\nPerhaps, it's good to run `ocean-drive auth` to updates your tokens.");
    }

    Ok(client)
}
