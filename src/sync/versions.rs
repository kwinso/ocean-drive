/*
    This struct is used to manage versions.json file
    It can be shared between threads and used with mutex to avoid threads trying to read/write to file simulteniosly
*/
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use serde_json;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VersionLog {
    pub is_folder: bool,
    pub parent_id: String,
    pub version: String,
    pub path: String,
    pub md5: Option<String>,
}

pub type VersionsList = std::collections::HashMap<String, VersionLog>;

pub struct Versions {
    path: PathBuf,
}

impl Versions {
    pub fn new(path: PathBuf) -> Result<Self> {
        // Check if file accessible
        match fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(&path)
        {
            Ok(_) => Ok(Self { path }),
            Err(e) => {
                bail!("Unable to access versions file, this file is required for program to work.\nDetails: {}", e);
            }
        }
    }

    pub fn list(&self) -> Result<VersionsList> {
        match fs::read_to_string(&self.path) {
            Ok(content) => match serde_json::from_str::<VersionsList>(content.as_str()) {
                Ok(r) => Ok(r),
                Err(_) => Ok(VersionsList::new()),
            },
            Err(e) => {
                bail!("Failed to read versions file. Details: {}", e);
            }
        }
    }

    pub fn save(&self, versions: VersionsList) -> Result<()> {
        match fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&self.path)
        {
            Ok(mut f) => {
                let content = serde_json::to_string(&versions).unwrap();
                match f.write_all(content.as_bytes()) {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        bail!("Failed to save versions data to file.\nDetails: {}", e);
                    }
                }
            }
            Err(e) => {
                bail!("Failed to access versions file.\nDetails: {}", e);
            }
        }
    }
}
