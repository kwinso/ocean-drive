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
    Setups two listeneres for updates: local and remote.
    Allows to share mutable google drive client between the two.
*/
pub fn run() -> Result<()> {
    let conf_dir = user::get_home()?.join(".config/ocean-drive");
    let conf_file = conf_dir.join("config.toml");

    let client = Arc::new(Mutex::new(setup_client(&conf_dir)?));
    let config = files::read_toml::<AppConfig>(conf_file)?;
    let versions = Arc::new(Mutex::new(Versions::new(conf_dir.join("versions.json"))?));

    // Todo: throw it in own thread + some error handling

    let cl = Arc::clone(&client);
    let ver = Arc::clone(&versions);
    let remote_watcher = thread::spawn(move || {
        let mut remote_manager = remote::RemoteManager::new(config, cl.clone(), ver).unwrap();
        remote_manager.start().unwrap();
    });

    let cloned = Arc::clone(&client);
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
        Err(_) => bail!("Try to run `ocean-drive auth` to update authorization data"),
    };

    let mut client = Client::new(
        creds.client_id.clone(),
        creds.client_secret.clone(),
        "https://localhost:8080".to_string(),
    );

    client.set_session(session.clone());

    // ToDo: Ask for authorization if unable to get new token
    if session.refresh_token.is_some() {
        match client.refresh_token() {
            Ok(new_session) => {
                files::write_toml(new_session, session_file)?;

                println!("Authorization for client is updated.");
            }
            Err(_) => bail!("Unable to update client refresh token"),
        };
    } else {
        println!("!! No refresh token for client is provided !!\nPerhaps, it's good to run `ocean-drive auth` to updates your tokens.");
    }

    Ok(client)
}
