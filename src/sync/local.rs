/*
    Handles local files updates (create, edit (also rename), delete)
    It'll upload updated files to the remote and make sure conficts are resolved by creating copy for files
*/
extern crate notify;

use crate::google_drive::Client;
use crate::setup::Config;
use crate::sync::versions::Versions;
use anyhow::{bail, Result};
use notify::{watcher, DebouncedEvent, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

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
                    let client = self.lock_client();

                    match event {
                        DebouncedEvent::Create(f) => {
                            self.create_file(f, &client, &versions)?;
                        }
                        _ => bail!("Event isn't implemented. ({:#?})", event),
                    }

                    drop(versions);
                    drop(client);
                }
                Err(e) => bail!("Unable to continue watching local files.\nDetails: {}", e),
            }
        }
    }

    fn create_file(
        &self,
        f: PathBuf,
        client: &MutexGuard<Client>,
        versions: &MutexGuard<Versions>,
    ) -> Result<()> {
        // Get file info from versions file
        let v_list = versions.list()?;
        let info = v_list
            .iter()
            .find(|&v| v.1.path == f.clone().into_os_string().into_string().unwrap());

        // File is completetly new
        if info.is_none() {
            // TODO:
            // if file with such name already exists on the cloud
            //      if so, then rename current file to .local version (and pass the execution to
            //      the next iteration)
            // else Get file mime type and then upload it to the cloud
            // then Add file to versions file
            println!("New file created");
        }

        Ok(())
    }
}
