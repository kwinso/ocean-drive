mod remote;

use crate::{
    auth::Creds,
    files,
    google_drive::{Client, Session},
    setup::Config as AppConfig,
    user,
};
use std::path::PathBuf;

/*
    Setups two listeneres for updates: local and remote.
    Allows to share mutable google drive client between the two.
*/
pub async fn run() -> Result<(), ()> {
    let conf_dir = user::get_home()?.join(".config/ocean-drive");
    let conf_file = conf_dir.join("config.toml");

    let client = setup_client(&conf_dir).await?;
    let config = files::read_toml::<AppConfig>(&conf_file)?;
    let mut remote_manager = remote::RemoteManager::new(config, client.clone()).await?;

    // ToDo: Create versions file if not exists

    // Todo: throw it in own thread + some error handling
    remote_manager.start().await;

    Ok(())
}

async fn setup_client(conf_dir: &PathBuf) -> Result<Client, ()> {
    let session_file = conf_dir.join("session.toml");
    let creds_file = conf_dir.join("creds.toml");

    let session;
    let creds = files::read_toml::<Creds>(&creds_file)?;

    match files::read_toml::<Session>(&session_file) {
        Ok(s) => {
            session = s;
        }
        Err(_) => {
            eprintln!("Try to run `ocean-drive auth` to update authorization data");
            return Err(());
        }
    };

    let mut client = Client::new(
        creds.client_id.clone(),
        creds.client_secret.clone(),
        "https://localhost:8080".to_string(),
    );

    client.set_session(session.clone());

    // ToDo: Ask for authorization if unable to get new token
    if session.refresh_token.is_some() {
        match client.refresh_token().await {
            Ok(new_session) => {
                files::write_toml(new_session, &session_file)?;

                println!("Authorization for client is updated.");
            }
            Err(_) => return Err(()),
        };
    } else {
        println!("!! No refresh token for client is provided !!\nPerhaps, it's good to run `ocean-drive auth` to updates your tokens.");
    }

    Ok(client)
}
