/*
    Handles local files updates (create, edit (also rename), delete)
    It'll upload updated files to the remote and make sure conficts are resolved by creating copy for files
*/
extern crate notify;

use crate::sync::util;
use crate::{
    files,
    google_drive::{types::File, Client},
    setup::Config,
    sync::versions::{Version, Versions, VersionsList},
};
use anyhow::{bail, Context, Result};
use chrono;
use md5;
use notify::{watcher, DebouncedEvent, RecursiveMode, Watcher};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{mpsc::channel, Arc, Mutex, MutexGuard},
    time::Duration,
};

pub struct LocalDaemon {
    client: Arc<Mutex<Client>>,
    root_path: PathBuf,
    remote_root_id: String,
    versions: Arc<Mutex<Versions>>,
}

impl LocalDaemon {
    pub fn new(
        config: Config,
        client: Arc<Mutex<Client>>,
        versions: Arc<Mutex<Versions>>,
        remote_dir_id: String,
    ) -> Result<Self> {
        let local_root = Path::new(&config.local_dir).to_path_buf();

        if !local_root.exists() {
            bail!(
                "Directory {:?} defined as the local syncing root for the app does not exist.",
                &config.local_dir
            );
        }

        return Ok(Self {
            versions,
            client,
            root_path: local_root,
            remote_root_id: remote_dir_id,
        });
    }

    pub fn start(&self) -> Result<()> {
        // Create a channel to receive the events.
        let (tx, rx) = channel();
        // Create a watcher object, delivering debounced events.
        // The notification back-end is selected based on the platform.
        let mut watcher = watcher(tx, Duration::from_secs(5)).context(
            "Failed to create a watcher object for getting updates from local directory.",
        )?;

        // Add a path to be watched. All files and directories at that path and
        // below will be monitored for changes.
        watcher
            .watch(&self.root_path, RecursiveMode::Recursive)
            .context("Failed to start receiving updates from local directory.")?;

        loop {
            let event = rx
                .recv()
                .context("Unable to continue getting updates from local folder")?;
            let client = util::lock_ref_when_free(&self.client);
            let mut versions = util::lock_ref_when_free(&self.versions);
            let mut v_list = versions.list()?;

            match &event {
                DebouncedEvent::Create(f) => {
                    self.handle_write(&f, &client, &mut v_list)?;
                }
                DebouncedEvent::Write(f) => {
                    if f.is_file() {
                        self.handle_write(&f, &client, &mut v_list)?;
                    }
                }
                DebouncedEvent::Rename(old, new) => {
                    let parent = new.parent().with_context(|| {
                        format!(
                            "Failed to get file parent on renamed file {:?}",
                            new.display()
                        )
                    })?;

                    self.handle_rename(
                        old.to_path_buf(),
                        new.to_path_buf(),
                        parent.to_path_buf(),
                        &client,
                        &mut v_list,
                    )?;
                }
                DebouncedEvent::Remove(f) => {
                    self.handle_delete(f.to_path_buf(), &client, &mut v_list)?
                }
                _ => {}
            }

            versions.save(v_list)?;

            drop(versions);
            drop(client);
        }
    }

    // TODO: Move to some utils mod
    /// Moves the file into the path prefixed with timestamp to differ it from other copies
    /// Returns `PathBuf` with new file
    /// This is an util function
    fn create_local_copy(&self, f: &PathBuf) -> Result<PathBuf> {
        let parent_path = f.parent();
        let name = self.get_file_name(f.clone())?;

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
        fs::rename(f, &new_path).with_context(|| {
            format!(
                "Error creating local copy {:?} for the file {:?},",
                new_path,
                f.display()
            )
        })?;

        Ok(Path::new(&new_path).to_path_buf())
    }

    /// Retrieves a readable file name
    /// In other cases it'll throw an error
    fn get_file_name(&self, f: &PathBuf) -> Result<&str> {
        let f = f.clone();
        let name = f
            .file_name()
            .with_context(|| format!("Unable to get file / directory name {:?}", f.display()))?
            .to_str()
            .clone();

        if name.is_none() {
            bail!(
                "Unable to read file / directory name {:?}. Perhaps, it has UTF-8 non-valid name",
                f.display()
            );
        }

        Ok(name.unwrap())
    }

    /// Handles logic for new and updated files
    fn handle_write(
        &self,
        f: &PathBuf,
        client: &MutexGuard<Client>,
        v_list: &mut VersionsList,
    ) -> Result<()> {
        if !f.exists() {
            return Ok(());
        }

        if let Some(parent) = f.parent() {
            // Get file info from versions file
            if f.is_file() {
                self.upload_file(f.to_path_buf(), parent.to_path_buf(), &client, v_list)?;
            }
            if f.is_dir() {
                self.upload_dir(f.to_path_buf(), parent.to_path_buf(), &client, v_list)?;
            }
            return Ok(());
        }

        println!("Warn: Failed to get file parent: {:?}", f.display());

        Ok(())
    }

    fn handle_rename(
        &self,
        old_file: PathBuf,
        new_file: PathBuf,
        parent: PathBuf,
        client: &MutexGuard<Client>,
        v_list: &mut VersionsList,
    ) -> Result<()> {
        if !new_file.exists() || !parent.exists() {
            return Ok(());
        }

        // When the parents is not in local directory, the file is moved somewhere outside of root
        // dir for the app (means cannot be synced anymore)
        if !parent
            .display()
            .to_string()
            .starts_with(&self.root_path.display().to_string())
        {
            return self.handle_delete(old_file, client, v_list);
        }

        // Get information about previous location of the file
        let old_info = Versions::find_item_by_path(old_file, v_list);

        if let Some(info) = old_info {
            let parent_id = if let Some(v) = Versions::find_item_by_path(parent.clone(), v_list) {
                v.0
            } else {
                self.remote_root_id.clone()
            };

            // Remove the old info about the file
            v_list.remove(&info.0);

            let new_name = self.get_file_name(&new_file)?;

            let updated = client.rename_file(info.0, new_name, parent_id.clone())?;

            // Save the new version (then remote daemon won't update this file again since it's
            // already in sync with the cloud)
            let new_v = Version {
                md5: updated.md5,
                path: new_file.display().to_string(),
                version: updated.version.unwrap_or(String::from("1")),
                is_folder: false,
                parent_id,
            };

            v_list.insert(updated.id.unwrap(), new_v);
        } else {
            // If file was not on versions list earlier, this file is completly new so handle it like a
            // new file
            self.handle_write(&new_file, client, v_list)?;
        }

        Ok(())
    }

    // Love this function, so small :+)
    fn handle_delete(
        &self,
        f: PathBuf,
        client: &MutexGuard<Client>,
        v_list: &mut VersionsList,
    ) -> Result<()> {
        if let Some(v) = Versions::find_item_by_path(f, v_list) {
            v_list.remove(&v.0);
            client.detele_file(v.0)?;
        }

        Ok(())
    }

    // Recursively upload every file or create a new dir
    fn upload_dir(
        &self,
        mut dir: PathBuf,
        parent: PathBuf,
        client: &MutexGuard<Client>,
        v_list: &mut VersionsList,
    ) -> Result<()> {
        // Don't upload already synced dir
        if let Some(_) = Versions::find_item_by_path(dir.clone(), v_list) {
            return Ok(());
        }

        let name = self.get_file_name(&dir)?;

        let parent_id = if let Some(info) = Versions::find_item_by_path(parent.clone(), v_list) {
            info.0
        } else {
            self.remote_root_id.clone()
        };

        if let Some(remote) = client.get_file_by_name(name, Some(parent_id.clone()))? {
            if !remote.trashed.unwrap_or(false) {
                let v = Versions::find_item_by_path(dir.clone(), v_list);
                // Creates a copy of the local directory if remote and local are different
                // or if there's no version in the versions file but we still get it untrashed in
                // the cloud
                if v.is_none() || v.as_ref().unwrap().1.version != remote.version.unwrap() {
                    let copy = self.create_local_copy(&dir);
                    if let Err(e) = copy {
                        println!("Warn: Was unable to create a local copy for the directory {:?}. This dir won't be uploaded to drive.", dir.display());
                        println!("Cause: {}", e);
                        return Ok(());
                    }
                    dir = copy.unwrap();

                    // Remove version if exists
                    if v.is_some() {
                        v_list.remove(&v.unwrap().0);
                    }
                }
            }
        }

        let new = client.create_dir(self.get_file_name(&dir)?, parent_id.clone())?;

        let v = Version {
            version: new.version.unwrap_or(String::from("1")),
            md5: None,
            path: dir.display().to_string(),
            is_folder: true,
            parent_id,
        };

        v_list.insert(new.id.unwrap(), v);

        // After we create a dir, we should upload all of it's children
        for f in fs::read_dir(&dir)? {
            let p = f
                .with_context(|| {
                    format!(
                        "Unable to read directory enrty when uploading directory {:?}",
                        dir.display()
                    )
                })?
                .path();

            if p.is_dir() {
                if let Err(e) = self.upload_dir(p, dir.clone(), client, v_list) {
                    println!("Failed to upload directory {:?}\nCause: {}", p.display(), e);
                }
            } else if p.is_file() {
                if let Err(e) = self.upload_file(p, dir.clone(), client, v_list) {
                    println!("Failed to upload file {:?}\nCause: {}", p.display(), e);
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

        if let Some(ref local) = local {
            // Update only if file is upadated compared to the old version
            // This check is needed because of the RemoteDaemon that can write to file and then
            // save the version. So when we meet write, it does not always mean the content was
            // updated
            if local.1.md5.as_ref().unwrap_or(&String::from("")) == &hash {
                return Ok(());
            }
        }

        let name = self.get_file_name(&f)?;

        let parent_id = if let Some(info) = Versions::find_item_by_path(parent.clone(), v_list) {
            info.0
        } else {
            self.remote_root_id.clone()
        };

        let remote_file = client.get_file_by_name(&name, Some(parent_id.clone()))?;

        // Check if the file on the remote is different from what we have on local
        if let Some(remote_file) = remote_file {
            if !remote_file.trashed.unwrap() {
                if let Some(md5) = remote_file.md5 {
                    if md5 == hash {
                        // Since file was new and it's already in the cloud, there's nothing to do
                        return Ok(());
                    }
                    f = self.create_local_copy(&f)?;
                }
            }
        }

        let new: File;

        if let Some(local) = local {
            // Remove old version from the versions list
            v_list.remove(&local.0);
            // And the upload the new on into the cloud
            new = client.update_file(local.0, content)?;
        } else {
            new = client.upload_file(self.get_file_name(&f)?, parent_id.clone(), content)?;
        }
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
