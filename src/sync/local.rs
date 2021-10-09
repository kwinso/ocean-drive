/*
    Handles local files updates (create, edit (also rename), delete)
    It'll upload updated files to the remote and make sure conficts are resolved by creating copy for files
*/
use crate::google_drive::Client;
use crate::setup::Config;
use crate::sync::versions::Versions;
use anyhow::{bail, Result};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

pub struct LocalDaemon {
    client_ref: Arc<Mutex<Client>>,
    config: Config,
    local_path: PathBuf,
    remote_dir_id: String,
    versions_ref: Arc<Mutex<Versions>>,
}

impl LocalDaemon {
    pub fn new(
        config: Config,
        client_ref: Arc<Mutex<Client>>,
        versions_ref: Arc<Mutex<Versions>>,
        remote_dir_id: String,
    ) -> Result<Self> {
        let local_path = Path::new(&config.local_dir).to_path_buf();

        if !local_path.exists() {
            bail!("Please, make your directory '{}' exists on your computer. You provided it's name as root where all files will be synced", &config.local_dir);
        }

        return Ok(Self {
            versions_ref,
            client_ref,
            config,
            local_path,
            remote_dir_id,
        });
    }

    pub fn start(&self) -> Result<()> {
        println!("Local Sync started");

        Ok(())
    }
}
