/*
    Handles local files updates (create, edit (also rename), delete)
    It'll upload updated files to the remote and make sure conficts are resolved by creating copy for files
*/
extern crate notify;

use crate::{
    files,
    google_drive::Client,
    setup::Config,
    sync::versions::{Version, Versions, VersionsItem, VersionsList},
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

    /// Function that contains all the logic for creating new file / directory
    fn handle_create(
        &self,
        f: &PathBuf,
        client: &MutexGuard<Client>,
        v_list: &mut VersionsList,
    ) -> Result<()> {
        // Get file info from versions file
        if f.is_file() {
            self.upload_new_file(f.to_path_buf(), &client, v_list)?;
        }
        if f.is_dir() {
            let paths = fs::read_dir(f)?;

            for p in paths {
                if let Ok(p) = p {
                    let p = p.path();
                    if p.is_file() {
                        self.upload_new_file(f.to_path_buf(), &client, v_list)?;
                    }
                    if p.is_dir() {
                        // TODO
                    }
                }
            }
        }

        Ok(())
    }

    fn upload_new_file(
        &self,
        mut file: PathBuf,
        client: &MutexGuard<Client>,
        v_list: &mut VersionsList,
    ) -> Result<()> {
        if file.is_dir() {
            return Ok(());
        }

        let info = Versions::find_item_by_path(file.clone(), v_list.clone())?;
        let content = files::read_bytes(file.to_path_buf())?;
        let hash = format!("{:x}", md5::compute(&content));

        if let Some(info) = info {
            // We will update version only if hashes do not match
            if info.1.md5.unwrap_or("".to_string()) != hash {
                v_list.remove(&info.0);
            } else {
                return Ok(());
            }
        }

        let file_name = file.file_name();

        // Something is wrong, probably it's good to ust skip this file
        if file_name.is_none() || file_name.unwrap().to_str().is_none() {
            return Ok(());
        }

        let mut file_name = file_name.unwrap().to_str().unwrap().to_string();

        let parent_path = file.parent();
        let mut parent_info: Option<VersionsItem> = None;

        if let Some(p) = parent_path {
            parent_info = Versions::find_item_by_path(p.to_path_buf(), v_list.clone())?;
        }

        let parent_id = if parent_info.is_some() {
            parent_info.unwrap().0
        } else {
            // Set root as the parent
            self.remote_root_id.clone()
        };

        let remote_file = client.get_file_by_name(&file_name, Some(&parent_id))?;

        // Check if the file on the remote is different from what we have on local
        if let Some(remote_file) = remote_file {
            if !remote_file.trashed.unwrap() {
                if remote_file.md5.unwrap() == hash {
                    // Since file was new and it's already in the cloud, there's nothing to do
                    return Ok(());
                }

                // We start path for the new file with it's parent
                let mut new_path = parent_path
                    .clone()
                    .unwrap_or(&Path::new("")) // It's root if it's no parent
                    .to_path_buf()
                    .into_os_string()
                    .into_string()
                    .unwrap();

                // Then we add slash because parent is a directory
                new_path.push_str("/");

                // New file name is built using 2 parts: time tag and the old name
                let t = chrono::Local::now();
                let mut new_name = t.format("[%d.%m.%y %H:%M:%S] ").to_string();
                new_name.push_str(&file_name);

                // Finish building new file path
                new_path.push_str(&new_name);

                // Move our file to the new path. Old path will be overwriten by remote daemon
                fs::rename(file.to_str().unwrap(), &new_path)?;

                println!(
                    "Info: Created new file to avoid duplicates '{}'.",
                    &new_path
                );

                // Assign function scope variables for futher actions such as uploading
                file_name = new_name;
                file = Path::new(&new_path).to_path_buf();
            }
        }

        let guessed = mime_guess::from_path(&file).first_or_octet_stream();
        let mime = guessed.essence_str();
        let file_path = file.into_os_string().into_string().unwrap();

        let uploaded = client.upload_file(&file_name, mime, parent_id.clone(), content)?;

        println!(
            "Info: New file '{}' was uploaded to the cloud. (Local path: '{}')",
            &file_name, &file_path
        );

        // Add information about the file to the versions file so it won't be proccessed twice
        let new_v = Version {
            md5: Some(hash),
            path: file_path,
            version: uploaded.version.unwrap_or(String::from("")),
            is_folder: false,
            parent_id,
        };

        v_list.insert(uploaded.id.unwrap(), new_v);

        Ok(())
    }
}
