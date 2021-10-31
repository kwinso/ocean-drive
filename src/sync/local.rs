/*
    Handles local files updates (create, edit (also rename), delete)
    It'll upload updated files to the remote and make sure conficts are resolved by creating copy for files
*/
extern crate notify;

use crate::{
    files,
    google_drive::Client,
    setup::Config,
    sync::versions::{Version, Versions, VersionsList},
};
use anyhow::{bail, Result};
use chrono;
use md5;
use mime_guess;
use notify::{watcher, DebouncedEvent, RecursiveMode, Watcher};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{mpsc::channel, Arc, Mutex, MutexGuard},
    time::Duration,
};

pub struct LocalDaemon {
    client_ref: Arc<Mutex<Client>>,
    config: Config,
    local_root: PathBuf,
    remote_root_id: String,
    versions_ref: Arc<Mutex<Versions>>,
}

// TODO:
// 1. file creation
//  1. upload to the cloud
//      1. If file already exists in the cloud, then make ".local" copy
//  2. Create version in file
// 2. File deletion
//  1. Delete from cloud
//      1. if local md5 in versions file != md5 for the file in the cloud, copy the file in the
//         cloud with .bak existion
//  2. Remove version from file
// 3. file renaming (also means moving)
//  if file was moved somewhere out of local root path, do step 2). "file deletion"
//  if file is folder => change all children for the dir in the versions file
//  rename file in the cloud
//  - if file was moved out of parent dir, get parent dir for new path, find in in versions file
//  and set parents to this file
impl LocalDaemon {
    pub fn new(
        config: Config,
        client_ref: Arc<Mutex<Client>>,
        versions_ref: Arc<Mutex<Versions>>,
        remote_dir_id: String,
    ) -> Result<Self> {
        let local_root = Path::new(&config.local_dir).to_path_buf();

        if !local_root.exists() {
            bail!("Please, make your directory '{}' exists on your computer. You provided it's name as root where all files will be synced", &config.local_dir);
        }

        return Ok(Self {
            versions_ref,
            client_ref,
            config,
            local_root,
            remote_root_id: remote_dir_id,
        });
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

    pub fn start(&self) -> Result<()> {
        // Create a channel to receive the events.
        let (tx, rx) = channel();
        // Create a watcher object, delivering debounced events.
        // The notification back-end is selected based on the platform.
        let mut watcher = watcher(tx, Duration::from_secs(10)).unwrap();

        // Add a path to be watched. All files and directories at that path and
        // below will be monitored for changes.
        watcher
            .watch(&self.local_root, RecursiveMode::Recursive)
            .unwrap();

        loop {
            match rx.recv() {
                Ok(event) => {
                    let versions = self.lock_versions();
                    let mut v_list = versions.list()?;
                    let client = self.lock_client();

                    match &event {
                        DebouncedEvent::Create(f) => {
                            self.handle_create(&f, &client, &mut v_list)?;
                        }
                        _ => println!("Event isn't implemented. ({:#?})", event),
                    }

                    versions.save(v_list)?;

                    drop(versions);
                    drop(client);
                }
                Err(e) => bail!("Unable to continue watching local files.\nDetails: {}", e),
            }
        }
    }

    /// Differ files and dirs and call needed function for creation
    fn handle_create(
        &self,
        f: &PathBuf,
        client: &MutexGuard<Client>,
        v_list: &mut VersionsList,
    ) -> Result<()> {
        println!("{:#?}", &f);
        if !f.exists() {
            return Ok(());
        }

        let parent = f
            .parent()
            .expect(&format!("Failed to get file parent: {:?}", f.display()));
        // Get file info from versions file
        if f.is_file() {
            self.upload_file(f.to_path_buf(), parent.to_path_buf(), &client, v_list)?;
        }
        if f.is_dir() {
            self.upload_dir(f.to_path_buf(), parent.to_path_buf(), &client, v_list)?;
        }

        Ok(())
    }

    /// Return `PathBuf` with new file
    fn create_local_copy(&self, f: &PathBuf) -> Result<PathBuf> {
        let parent_path = f.parent();
        let name = f
            .file_name()
            .expect(&format!(
                "Unable to identify name for the file {:?}",
                f.display()
            ))
            .to_str()
            .unwrap();

        // We start path for the new file with it's parent
        let mut new_path = parent_path
            .clone()
            .unwrap_or(&Path::new("")) // It's root if it's no parent
            .to_path_buf()
            .display()
            .to_string();

        // Then we add slash because parent is a directory
        new_path.push_str("/");

        // New file name is built using 2 parts: time tag and the old name
        let t = chrono::Local::now();
        let mut new_name = t.format("[%d.%m.%y %H:%M:%S] ").to_string();
        new_name.push_str(&name);

        // Finish building new file path
        new_path.push_str(&new_name);

        // Move our file to the new path. Old path will be overwriten by remote daemon
        fs::rename(f, &new_path)?;

        Ok(Path::new(&new_path).to_path_buf())
    }

    // Recursivelly for through every file
    fn upload_dir(
        &self,
        mut dir: PathBuf,
        parent: PathBuf,
        client: &MutexGuard<Client>,
        v_list: &mut VersionsList,
    ) -> Result<()> {
        let d = dir.clone();
        let name = d.file_name().unwrap().to_str().clone().expect(&format!(
            "Unable to read directory name {:?}",
            dir.display()
        ));

        let parent_id = if let Some(info) = Versions::find_item_by_path(parent.clone(), v_list) {
            info.0
        } else {
            self.remote_root_id.clone()
        };

        if let Some(remote) = client.get_file_by_name(name, Some(parent_id.clone()))? {
            if !remote.trashed.unwrap_or(false) {
                let v = Versions::find_item_by_path(dir.clone(), v_list);

                if v.is_none() || v.as_ref().unwrap().1.version != remote.version.unwrap() {
                    // Remove version if exists
                    if v.is_some() {
                        v_list.remove(&v.unwrap().0);
                    }
                    dir = self.create_local_copy(&dir)?;
                }
            }
        }

        let new = client.create_dir(
            dir.file_name()
                .unwrap()
                .to_str()
                .unwrap_or("[non-readable name]"),
            parent_id.clone(),
        )?;
        let v = Version {
            version: new.version.unwrap_or(String::from("1")),
            md5: None,
            path: dir.display().to_string(),
            is_folder: true,
            parent_id,
        };

        v_list.insert(new.id.unwrap(), v);

        for f in fs::read_dir(&dir)? {
            let p = f?.path();
            if p.is_dir() {
                self.upload_dir(p, dir.clone(), client, v_list)?;
            } else {
                if p.is_file() {
                    self.upload_file(p, dir.clone(), client, v_list)?;
                }
            }
        }

        Ok(())
    }

    /// Uploading completly new file
    fn upload_file(
        &self,
        mut f: PathBuf,
        parent: PathBuf,
        client: &MutexGuard<Client>,
        v_list: &mut VersionsList,
    ) -> Result<()> {
        if f.is_dir() {
            return Ok(());
        }

        let local = Versions::find_item_by_path(f.clone(), v_list);
        let content = files::read_bytes(f.to_path_buf())?;
        let hash = format!("{:x}", md5::compute(&content));

        if let Some(local) = local {
            // Update only if file is upadated compared to the old version
            if local.1.md5.unwrap_or("".to_string()) != hash {
                v_list.remove(&local.0);
            } else {
                return Ok(());
            }
        }

        let c = f.clone();
        let name = c
            .file_name()
            .unwrap()
            .to_str()
            .expect(&format!("Unable to read file name {:?}", f.display()));

        let parent_id = if let Some(info) = Versions::find_item_by_path(parent.clone(), v_list) {
            info.0
        } else {
            self.remote_root_id.clone()
        };

        let remote_file = client.get_file_by_name(&name, Some(parent_id.clone()))?;

        // Check if the file on the remote is different from what we have on local
        if let Some(remote_file) = remote_file {
            if !remote_file.trashed.unwrap() {
                if remote_file.md5.is_some() && remote_file.md5.unwrap() == hash {
                    // Since file was new and it's already in the cloud, there's nothing to do
                    return Ok(());
                }
            }

            f = self.create_local_copy(&f)?;
        }
        let guessed = mime_guess::from_path(&f).first_or_octet_stream();
        let mime = guessed.essence_str();

        let new = client.upload_file(
            f.file_name()
                .unwrap()
                .to_str()
                .unwrap_or("[non-readable name]"),
            mime,
            parent_id.clone(),
            content,
        )?;

        // Add information about the file to the versions file so it won't be proccessed twice
        let new_v = Version {
            md5: new.md5,
            path: f.display().to_string(),
            version: new.version.unwrap_or(String::from("1")),
            is_folder: false,
            parent_id,
        };

        v_list.insert(new.id.unwrap(), new_v);

        Ok(())
    }
}
