/* Handle local & remote update and sync them */

use notify::{watcher, RecursiveMode, Watcher};
use std::{
    path::{Path, PathBuf},
    sync::mpsc::channel,
    time::Duration,
    process::exit
};
use crate::{
    google_drive::{Client, Session},
    user,
    files,
    auth::Creds,
    setup::Config
};

pub struct Updates {
    client: Client,
}

impl Updates {
    pub async fn new() -> Result<Self, String> {
        let conf_dir = user::get_home()?.join(".config/ocean-drive");
        let conf_file = conf_dir.join("config.toml");

        let conf = files::read_toml::<Config>(&conf_file)?;
        let client = Updates::setup_client(&conf_dir).await?;       


        let list = client.list_files(None).await?;

        for file in list.files {
            println!("{:#?}", file.name);
        }

        Ok(Self{ client })
    }

    async fn setup_client(conf_dir: &PathBuf) -> Result<Client, String> {
        let session_file = conf_dir.join("session.toml");
        let creds_file = conf_dir.join("creds.toml");


        let session = files::read_toml::<Session>(&session_file)?;
        let creds = files::read_toml::<Creds>(&creds_file)?;

        let mut client = Client::new(
            creds.client_id.clone(), 
            creds.client_secret.clone(),
            "https://localhost:8080".to_string()
        );


        client.set_session(session.clone());
        // Update access token if exists. Save it to the file
        if let Some(refresh_token) = session.refresh_token {
            let new_session = client.get_new_session(refresh_token).await?;
            if new_session.is_some() {
                let new_session = new_session.unwrap();
                client.set_session(new_session.clone());
                files::write_toml(new_session, &session_file)?;
            } 
        }

        Ok(client)
    }

    // pub async fn watch(self) -> Result<(), String> {
    //     let gdrive = DriveClient::new().await?;
    //     let dirs = gdrive.client.query_files(&format!("name = '{}' and mimeType = 'application/vnd.google-apps.folder'", gdrive.config.dir)).await?;

    //     if dirs.len() == 0 {
    //         return Err(format!("Directory with name '{}' is not found in your drive", gdrive.config.dir))
    //     }


    //     let root_id = dirs[0].clone().id;
    //     let root = gdrive.client.get_file(&root_id).await?;

    //     println!("{:#?}", root);

    //     // let dir_files = gdrive.list_files_with_query(&format!("'{}' in parents", dir.id)).await?;

    //     // println!("{:#?}", dir_files);

    //     Ok(())
    // }
    // fn watch_drive(drive: GoogleDrive) {

    // } 
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
