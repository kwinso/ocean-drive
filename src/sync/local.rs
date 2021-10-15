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
    remote_dir_id: String,
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
            remote_dir_id,
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

                    match event {
                        DebouncedEvent::Create(f) => {
                            self.create_file(f, &client, &mut v_list)?;
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

    fn create_file(
        &self,
        mut file: PathBuf,
        client: &MutexGuard<Client>,
        versions: &mut VersionsList,
    ) -> Result<()> {
        // Get file info from versions file
        let info = Versions::find_item_by_path(file.clone(), versions.clone())?;

        // File is completetly new
        if info.is_none() {
            // TODO:
            // if file with such name already exists on the cloud
            //      rename current file to .local version (and pass the execution to
            //      the next iteration)
            // else Get file mime type and then upload it to the cloud
            // then Add file to versions file
            let name = file.file_name();

            // Something is wrong, probably it's good to ust skip this file
            if name.is_none() || name.unwrap().to_str().is_none() {
                return Ok(());
            }

            let mut name = name.unwrap().to_str().unwrap().to_string();

            println!("Info: Spotted new file {}.", file.to_str().unwrap());

            let parent_path = file.parent();
            let mut parent_info: Option<VersionsItem> = None;

            if let Some(path) = parent_path {
                parent_info = Versions::find_item_by_path(path.to_path_buf(), versions.clone())?;
            }

            let parent_id = if parent_info.is_some() {
                parent_info.unwrap().0
            } else {
                self.remote_dir_id.clone()
            };

            println!("Info: Recognized file parent id: '{}'", &parent_id);

            let contents = files::read_bytes(file.to_path_buf())?;
            let remote_file = client.get_file_by_name(&name, Some(&parent_id))?;

            // Check if the file on the remote is different from what we have on local
            if let Some(remote_file) = remote_file {
                let local_md5 = md5::compute(&contents);

                // TODO: Fix troubles with comparing md5 (it's differs in the cloud)
                if !remote_file.trashed.unwrap()
                    && remote_file.md5.unwrap() != format!("{:x}", local_md5).to_string()
                {
                    println!(
                        "Info: Spotted file with duplicate name in the remote. Making local copy."
                    );
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

                    // New file name is built from time tag and the old name
                    let t = chrono::Local::now();
                    let mut new_name = t.format("[%d.%m.%y %H:%M:%S] ").to_string();
                    new_name.push_str(&name);

                    // Finish building new file path
                    new_path.push_str(&new_name);

                    // Move our file to the new path. Old path will be overwriten by remote file
                    fs::rename(file.to_str().unwrap(), &new_path)?;

                    println!("Info: Created file '{}'.", &new_path);

                    // Assign function scope variables for futher actions such as uploading
                    name = new_name;
                    file = Path::new(&new_path).to_path_buf();
                }
            }

            let guessed = mime_guess::from_path(&file).first_or_octet_stream();
            let mime = guessed.essence_str();

            let uploaded = client.upload_file(&name, mime, parent_id.clone(), contents)?;

            // Add information about the file to the versions file so it won't be proccessed twice
            let new_v = Version {
                md5: uploaded.md5,
                path: file.into_os_string().into_string().unwrap(),
                version: uploaded.version.unwrap_or(String::from("")),
                is_folder: false,
                parent_id,
            };

            versions.insert(uploaded.id.unwrap(), new_v);
        }

        Ok(())
    }
}
