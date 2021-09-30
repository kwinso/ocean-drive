/*
    Handles all the logic about handling updates from the remote drive, uploading and downloading files
    from remote to local
*/
use crate::google_drive::{errors::DriveClientError, types::File, Client};
use crate::setup::Config;
use crate::sync::versions::VersionLog;
use crate::sync::Versions;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use std::{
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};
use std::{io::Write, sync::MutexGuard};

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
        let client = client_ref.lock().unwrap();

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

    fn lock_versions(&self) -> MutexGuard<Versions> {
        loop {
            if let Ok(versions) = self.versions.try_lock() {
                return versions;
            }
        }
    }

    fn lock_client(&self) -> MutexGuard<Client> {
        loop {
            if let Ok(client) = self.client_ref.try_lock() {
                return client;
            }
        }
    }

    pub fn start(&mut self) -> Result<(), ()> {
        loop {
            let versions = self.lock_versions();
            let mut versions_list = versions.list().unwrap();
            let mut client = self.lock_client();

            match self.sync_dir(
                self.remote_dir_id.clone(),
                PathBuf::from_str(&self.config.local_dir).unwrap(),
                &client,
                &mut versions_list,
            ) {
                Ok(_) => {}
                Err(e) => match e {
                    DriveClientError::Unauthorized => {
                        match client.refresh_token() {
                            Err(_) => {
                                eprintln!("Failed to update authorization data for app.\nPlease run `ocean-drive auth` to renew authorization data and then start the program again.");
                                return Err(());
                            }
                            Ok(_) => {
                                // Todo: Add counter for attempts
                                println!("Refresh token is updating. Trying to fetch files again");
                                drop(client);
                            }
                        };
                        continue;
                    }
                    _ => return Err(()),
                },
            }

            versions.save(versions_list).unwrap();
            // Make shared references avaliable again
            drop(versions);
            drop(client);

            std::thread::sleep(std::time::Duration::from_secs(10));
        }
    }

    // Todo: fix conflicts between error types (reduce amount of unwraps)
    // Todo: Files do not move if folder name was changed
    // Todo: Just rename file / folder if everthing else is ok 
    fn sync_dir(
        &self,
        id: String,
        dir_path: PathBuf,
        drive: &MutexGuard<Client>,
        local_versions: &mut HashMap<String, VersionLog>,
    ) -> Result<(), DriveClientError> {
        let dir_info = drive.get_file_info(&id)?;
        let local_dir_info = local_versions.get(&id);

        // if the dir wasnt updated, then there's no need to even check this dir
        if local_dir_info.is_some() && dir_info.version.unwrap() == local_dir_info.unwrap().version
        {
            return Ok(());
        }

        let dir = drive.list_files(
            Some(&format!("'{}' in parents", &id)),
            Some("files(id, name, trashed, mimeType, parents, version)"),
        )?;
        // Files is a haspmap with key of file id and value is file
        let mut files: HashMap<String, File> = HashMap::new();
        dir.files.iter().for_each(|f| {
            files.insert(f.id.as_ref().unwrap().clone(), f.clone());
        });

        for (file_id, file) in files.clone() {
            let is_folder =
                file.mime_type.as_ref().unwrap() == "application/vnd.google-apps.folder";
            let v = local_versions.clone();
            let local = v.get(&file_id);

            // This file is new or changed
            if local.is_none() || &local.unwrap().version != file.version.as_ref().unwrap() {
                if file.trashed.unwrap() {
                    local_versions.remove(&file_id);
                    if let Some(local) = local {
                        let removed_path = Path::new(&local.path);
                        if removed_path.exists() {
                            if is_folder {
                                fs::remove_dir_all(&removed_path).unwrap();
                            } else {
                                fs::remove_file(&removed_path).unwrap();
                            }
                        }
                    }
                    continue;
                }
                // If changed we need to update existing one. We need to remove existing for it
                if is_folder {
                    let subdir = dir_path.join(file.name.as_ref().unwrap());
                    if !subdir.exists() {
                        fs::create_dir(subdir.clone()).unwrap();
                    }
                    self.sync_dir(file.clone().id.unwrap(), subdir, drive, local_versions)?;
                } else {
                    if local.is_some() {
                        let old_path = Path::new(&local.unwrap().path);
                        if old_path.exists() {
                            fs::remove_file(old_path).unwrap();
                        }
                    }

                    let filepath = dir_path.join(&file.name.clone().unwrap());
                    self.save_file(drive, &file, &filepath).unwrap();
                }

                if local.is_some() {
                    local_versions.remove(&file_id);
                }

                let newest_version = VersionLog {
                    is_folder,
                    parent_id: id.clone(),
                    path: dir_path
                        .join(file.name.as_ref().unwrap())
                        .into_os_string()
                        .into_string()
                        .unwrap(),
                    version: file.version.as_ref().unwrap().to_string(),
                };
                local_versions.insert(file_id.clone(), newest_version.clone());
            }
        }

        Ok(())
    }

    fn save_file(
        &self,
        drive: &MutexGuard<Client>,
        file: &File,
        filepath: &PathBuf,
    ) -> Result<(), ()> {
        let contents = drive.download_file(file.id.as_ref().unwrap()).unwrap();

        if filepath.exists() {
            // Todo: maybe some better error handling for this?
            fs::remove_file(filepath).unwrap();
        }

        let mut file = fs::File::create(filepath).unwrap();

        match file.write(&contents) {
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
