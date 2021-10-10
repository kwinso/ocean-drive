/*
    Contains all the logic about handling updates from the remote drive, uploading and downloading files
    from remote to local
*/
use crate::auth;
use crate::google_drive::{errors::DriveError, types::File, Client};
use crate::setup::Config;
use crate::sync::versions::{VersionLog, Versions};
use anyhow::{bail, Result};
use std::{
    collections::HashMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    str::FromStr,
    sync::{Arc, Mutex, MutexGuard},
};

pub struct RemoteDaemon {
    client_ref: Arc<Mutex<Client>>,
    config: Config,
    remote_dir_id: String,
    versions_ref: Arc<Mutex<Versions>>,
}

// TODO: Escape bad characters when creating file path
impl RemoteDaemon {
    pub fn new(
        config: Config,
        client_ref: Arc<Mutex<Client>>,
        versions_ref: Arc<Mutex<Versions>>,
        remote_dir_id: String,
    ) -> Result<Self> {
        Ok(Self {
            versions_ref,
            client_ref,
            config,
            remote_dir_id,
        })
    }

    fn lock_versions(&self) -> MutexGuard<Versions> {
        loop {
            if let Ok(versions) = self.versions_ref.try_lock() {
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

    pub fn start(&mut self) -> Result<()> {
        loop {
            let versions = self.lock_versions();
            let mut versions_list = versions.list().unwrap();
            let mut client = self.lock_client();

            match self.sync_dir(
                &self.remote_dir_id,
                PathBuf::from_str(&self.config.local_dir).unwrap(),
                &client,
                &mut versions_list,
            ) {
                Ok(_) => {}
                Err(e) => {
                    if let Ok(err) = e.downcast::<DriveError>() {
                        match err {
                            DriveError::Unauthorized => {
                                match auth::update_for_shared_client(&mut client) {
                                    Ok(_) => {
                                        println!("Info: Client authorization was updated since it was out of date.");
                                        drop(client);
                                        continue;
                                    }
                                    Err(e) => bail!(e),
                                }
                            }
                        }
                    }

                    bail!("Unable to get updates from remote.");
                }
            }

            versions.save(versions_list).unwrap();
            // Make shared references avaliable again
            drop(versions);
            drop(client);

            std::thread::sleep(std::time::Duration::from_secs(10));
        }
    }

    fn sync_dir(
        &self,
        id: &String,
        dir_path: PathBuf,
        drive: &MutexGuard<Client>,
        local_versions: &mut HashMap<String, VersionLog>,
    ) -> Result<()> {
        let dir_info = drive.get_file_info(&id)?;
        let local_dir_info = local_versions.get(id);

        // if the dir wasnt updated, then there's no need to even check this dir
        if local_dir_info.is_some() && local_dir_info.unwrap().version == dir_info.version.unwrap()
        {
            return Ok(());
        }

        let dir = drive.list_files(
            Some(&format!("'{}' in parents", &id)),
            Some("files(id, md5Checksum, name, trashed, mimeType, parents, version)"),
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

            if local.is_some() {
                let local_path = Path::new(&local.unwrap().path);

                if !local_path.starts_with(&dir_path) {
                    let updated_path = dir_path.join(file.name.as_ref().unwrap());
                    let mut updated_version = local.unwrap().clone();
                    updated_version.path = updated_path.into_os_string().into_string().unwrap();
                    local_versions.remove(&file_id);
                    local_versions.insert(file_id.clone(), updated_version);
                }
            }

            // This file is new or changed
            if local.is_none() || &local.unwrap().version != file.version.as_ref().unwrap() {
                let name = file.name.as_ref().unwrap();
                let file_path = dir_path.join(name).to_path_buf();
                let file_path = file_path.to_str().unwrap();

                if file.trashed.unwrap() {
                    local_versions.remove(&file_id);
                    self.remove_from_fs(&local)?;
                    continue;
                }

                // If changed we need to update existing one. We need to remove existing for it
                if is_folder {
                    // Check directory name was changed, then just rename in on the file system
                    if let Some(local) = local {
                        if &local.path != file_path {
                            match fs::rename(&local.path, file_path) {
                                Err(e) => bail!(e),
                                Ok(_) => {}
                            }
                        }
                    }

                    // Generate a path for a subdirectory
                    let subdir = dir_path.join(name);
                    if !subdir.exists() {
                        fs::create_dir(subdir.clone())?;
                    }

                    // We go recursively for every file in the subdir
                    self.sync_dir(&file_id, subdir, drive, local_versions)?;
                } else {
                    // Check if it's a new file and download it
                    // Also re-download if we the file data has changed
                    if local.is_none() || local.unwrap().md5 != file.md5 {
                        let filepath = dir_path.join(&name);
                        self.save_file(drive, &file, filepath)?;
                    }

                    // If the file is present, we check if it's was renamed
                    if let Some(local) = local {
                        if &local.path != file_path {
                            fs::rename(&local.path, &file_path)?;
                        }
                    }
                }

                // If local version is present, we need to remove it before updating
                if local.is_some() {
                    local_versions.remove(&file_id);
                }

                let latest = VersionLog {
                    is_folder,
                    md5: file.md5,
                    parent_id: id.clone(),
                    path: dir_path.join(name).into_os_string().into_string().unwrap(),
                    version: file.version.as_ref().unwrap().to_string(),
                };
                local_versions.insert(file_id, latest.clone());
            }
        }

        Ok(())
    }

    fn save_file(&self, drive: &MutexGuard<Client>, file: &File, filepath: PathBuf) -> Result<()> {
        let contents = drive.download_file(file.id.as_ref().unwrap()).unwrap();

        match fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&filepath)
        {
            Ok(mut file) => {
                if let Err(e) = file.write(&contents) {
                    bail!(
                        "Unable write to file. (File: '{}')\nDetails: {}",
                        filepath.into_os_string().into_string().unwrap(),
                        e
                    )
                }

                Ok(())
            }
            Err(e) => bail!(
                "Unable access file. (File: '{}')\nDetails: {}",
                filepath.into_os_string().into_string().unwrap(),
                e
            ),
        }
    }

    /* Removes a file from a local root, the opposite of save_file fn */
    fn remove_from_fs(&self, local: &Option<&VersionLog>) -> Result<()> {
        if let Some(local) = local {
            let removed_path = Path::new(&local.path);

            if removed_path.exists() {
                if local.is_folder {
                    fs::remove_dir_all(&removed_path)?;
                } else {
                    fs::remove_file(&removed_path)?;
                }
            }
        }

        Ok(())
    }
}
