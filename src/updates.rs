/* Handle local & remote update and sync them */

use crate::{
    auth::Creds,
    files,
    google_drive::{Client, Session},
    setup::Config,
    user,
};
use notify::{watcher, RecursiveMode, Watcher};
use std::{
    path::{Path, PathBuf},
    process::exit,
    sync::mpsc::channel,
    time::Duration,
};

pub struct Updates {
    client: Client,
}

impl Updates {
    pub async fn new() -> Result<Self, ()> {
        let conf_dir = user::get_home()?.join(".config/ocean-drive");
        let conf_file = conf_dir.join("config.toml");

        let conf = files::read_toml::<Config>(&conf_file)?;
        let client = Updates::setup_client(&conf_dir).await?;

        // ToDo: Well, the next thing to do, probably
        // let list = client.list_files(None).await?;

        // for file in list.files {
        //     println!("{:#?}", file.name);
        // }

        Ok(Self { client })
    }

    async fn setup_client(conf_dir: &PathBuf) -> Result<Client, ()> {
        let session_file = conf_dir.join("session.toml");
        let creds_file = conf_dir.join("creds.toml");

        let session;
        let creds = files::read_toml::<Creds>(&creds_file)?;

        match files::read_toml::<Session>(&session_file) {
            Ok(s) => { session = s; }
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

        // ToDo: Ask for authorization if unable to get new token
        if session.refresh_token.is_some() {
            let new_session = client.update_auth(session.refresh_token.unwrap()).await;
            println!("Authorization for client is updated.");
            
            files::write_toml(new_session, &session_file)?;
        } else {
            println!("!! No refresh token for client is provided !!\nPerhaps, it's good to run `ocean-drive auth` to updates your tokens.");
            client.set_session(session);
        }
        

        Ok(client)
    }

    fn watch_local(path: String) {
        if !Path::new(&path).is_dir() {
            // log::error(format!("Root directory {:?} does not exist", path));
            exit(1);
        }
        // Create a channel to receive the events.
        let (tx, rx) = channel();

        // Create a watcher object, delivering debounced events.
        // The notification back-end is selected based on the platform.
        let mut watcher = watcher(tx, Duration::from_secs(10)).unwrap();

        // Add a path to be watched. All files and directories at that path and
        // below will be monitored for changes.
        watcher.watch(path, RecursiveMode::Recursive).unwrap();

        loop {
            match rx.recv() {
                Ok(event) => println!("{:?}", event),
                Err(e) => println!("watch error: {:?}", e),
            }
        }
    }
}
