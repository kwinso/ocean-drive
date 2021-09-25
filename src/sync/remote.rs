/*
    Handles all the logic about handling updates from the remote drive, uploading and downloading files
    from remote to local
*/
use crate::google_drive::{errors::DriveClientError, Client};
use crate::setup::Config;
use async_recursion::async_recursion;
use std::io::Write;
use std::{fs, path::PathBuf, str::FromStr};
use tokio::time;

pub struct RemoteManager {
    client: Client,
    config: Config,
    remote_dir_id: String,
}

// Todo: Compare with versions ffile and remove deletes files not to leave trash
impl RemoteManager {
    pub async fn new(config: Config, client: Client) -> Result<Self, ()> {
        match client
            .list_files(
                Some(&format!(
                    "name = '{}' and mimeType = 'application/vnd.google-apps.folder'",
                    config.drive.dir
                )),
                Some("files(id)"),
            )
            .await
        {
            Ok(list) => {
                if list.files.len() == 0 {
                    eprintln!(
                        "Folder with name '{}' not found in the root of your drive.",
                        config.drive.dir
                    );
                    return Err(());
                }
                let root_dir = list.files[0].clone();

                Ok(Self {
                    client,
                    config,
                    remote_dir_id: root_dir.id.unwrap(),
                })
            }
            Err(e) => {
                match e {
                    DriveClientError::Unauthorized => {
                        // todo: re-authorize
                    }
                    _ => {}
                }

                return Err(());
            }
        }
    }

    pub async fn start(&mut self) -> Result<(), ()> {
        let mut interval = time::interval(time::Duration::from_secs(6 * 3));
        loop {
            match self
                .download_dir(
                    self.remote_dir_id.clone(),
                    PathBuf::from_str(&self.config.local_dir).unwrap(),
                )
                .await
            {
                Ok(_) => {}
                Err(e) => match e {
                    DriveClientError::Unauthorized => {
                        match self.client.refresh_token().await {
                            Err(_) => {
                                eprintln!("Failed to update authorization data for app.\nPlease run `ocean-drive auth` to renew authorization data and then start the program again.");
                                return Err(());
                            }
                            Ok(_) => {
                                // Todo: Add counter for attempts
                                println!("Refresh token is updating. Trying to fetch files again");
                                continue;
                            }
                        };
                    }
                    _ => return Err(()),
                },
            }
            interval.tick().await;
        }
    }

    #[async_recursion]
    async fn download_dir(&self, id: String, local_path: PathBuf) -> Result<(), DriveClientError> {
        let dir = self
            .client
            .list_files(
                Some(&format!("'{}' in parents", id)),
                Some("files(id, name, mimeType, parents, version)"),
            )
            .await?;

        for file in dir.files {
            if file.mime_type.unwrap() == "application/vnd.google-apps.folder" {
                let subdir = local_path.join(file.name.unwrap());
                if !subdir.exists() {
                    fs::create_dir(subdir.clone()).unwrap();
                }
                self.download_dir(file.id.unwrap(), subdir).await?;
            } else {
                // Todo: Work on exception for every step
                let filepath = local_path.join(file.name.unwrap());

                let resp = self.client.download_file(file.id.unwrap()).await?;

                if filepath.exists() {
                    // Todo: maybe some better error handling for this?
                    match fs::remove_file(filepath.clone()) {
                        Ok(_) => {}
                        Err(e) => {
                            eprintln!(
                                "Failed to remove file '{}'.\nError: {}",
                                filepath.to_str().unwrap(),
                                e
                            );
                            continue;
                        }
                    }
                }

                let mut file = fs::File::create(filepath.clone()).unwrap();
                // let mut contVent = Cursor::new(resp);

                match file.write(&resp) {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!(
                            "[Error] File '{}' can't be saved.\nError: {}",
                            filepath.to_str().unwrap(),
                            e
                        );
                        fs::remove_file(filepath).unwrap();
                        continue;
                    }
                }
            }
        }

        Ok(())
    }
}
