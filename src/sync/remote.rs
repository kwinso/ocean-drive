/*
    Handles all the logic about handling updates from the remote drive, uploading and downloading files
    from remote to local
*/
use crate::google_drive::{errors::DriveClientError, types::{File, FileList}, Client};
use crate::setup::Config;
use crate::sync::Versions;
use crate::sync::versions::VersionLog;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::{fs, path::PathBuf, str::FromStr};

pub struct RemoteManager {
    client_ref: Arc<Mutex<Client>>,
    config: Config,
    remote_dir_id: String,
    versions: Arc<Mutex<Versions>>,
}

// Todo: Compare with versions ffile and remove deletes files not to leave trash
impl RemoteManager {
    pub fn new(
        config: Config,
        client_ref: Arc<Mutex<Client>>,
        versions: Arc<Mutex<Versions>>,
    ) -> Result<Self, ()> {
        let client = client_ref.try_lock().unwrap();

        match client.list_files(
            Some(&format!(
                "name = '{}' and mimeType = 'application/vnd.google-apps.folder'",
                config.drive.dir
            )),
            Some("files(id)"),
        ) {
            Ok(list) => {
                if list.files.len() == 0 {
                    eprintln!(
                        "Folder with name '{}' not found in the root of your drive.",
                        config.drive.dir
                    );
                    return Err(());
                }
                let root_dir = list.files[0].clone();

                // unlock client for other threads
                drop(client);

                Ok(Self {
                    versions,
                    client_ref,
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

    pub fn start(&mut self) -> Result<(), ()> {
        loop {
            match self.download_dir(
                self.remote_dir_id.clone(),
                PathBuf::from_str(&self.config.local_dir).unwrap(),
            ) {
                Ok(_) => {}
                Err(e) => match e {
                    DriveClientError::Unauthorized => {
                        // Trying to wait for the client to be accessible
                        loop {
                            if let Ok(mut client) = self.client_ref.lock() {
                                match client.refresh_token() {
                                    Err(_) => {
                                        eprintln!("Failed to update authorization data for app.\nPlease run `ocean-drive auth` to renew authorization data and then start the program again.");
                                        return Err(());
                                    }
                                    Ok(_) => {
                                        // Todo: Add counter for attempts
                                        println!("Refresh token is updating. Trying to fetch files again");
                                        drop(client);
                                        break;
                                    }
                                };
                            }
                        }
                        continue;
                    }
                    _ => return Err(()),
                },
            }
            std::thread::sleep(std::time::Duration::from_secs(60 * 3));
        }
    }

    // Todo: Loop through versions, if id from version no longer exists, delete this file
    // Todo: Remove file if it's in trash
    // Todo: Folders versioning
    // Todo: fix conflicts between error types (reduce amount of unwraps)
    fn download_dir(&self, id: String, local_path: PathBuf) -> Result<(), DriveClientError> {
        loop {
            if let Ok(client) = self.client_ref.try_lock() {
                let dir = client.list_files(
                    Some(&format!("'{}' in parents", &id)),
                    Some("files(id, name, mimeType, parents, version)"),
                )?;

                for file in dir.files {
                    if file.mime_type.as_ref().unwrap() == "application/vnd.google-apps.folder" {
                        let subdir = local_path.join(file.name.unwrap());
                        if !subdir.exists() {
                            fs::create_dir(subdir.clone()).unwrap();
                        }
                        self.download_dir(file.id.unwrap(), subdir)?;
                    } else {
                        loop {
                            if let Ok(versions) = self.versions.try_lock() {
                                if let Some(local) = versions.get(id.clone()).unwrap() {
                                    if &local.version != file.version.as_ref().unwrap() {
                                        let contents = client.download_file(id.clone())?;
                                        self.update_file(&file, contents, &local_path).unwrap();
                                    }
                                } else {
                                    let contents = client.download_file(id.clone())?;
                                    self.update_file(&file, contents, &local_path).unwrap();
                                    versions.set(id.clone(), VersionLog { version: file.version.unwrap() }).unwrap();
                                }
                                drop(versions);
                                break;
                            }
                        }
                    }
                }

                drop(client);
                break;
            }
        }

        Ok(())
    }

    fn update_file(
        &self,
        file: &File,
        content: bytes::Bytes,
        parent_path: &PathBuf,
    ) -> Result<(), ()> {
        let filepath = parent_path.join(file.name.as_ref().unwrap());

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
                    return Err(());
                }
            }
        }

        let mut file = fs::File::create(filepath.clone()).unwrap();

        match file.write(&content) {
            Ok(_) => Ok(()),
            Err(e) => {
                eprintln!(
                    "[Error] File '{}' can't be saved.\nError: {}",
                    filepath.to_str().unwrap(),
                    e
                );
                fs::remove_file(filepath).unwrap();
                return Err(());
            }
        }
    }
}
