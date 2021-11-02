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

/// Represents all data assosiated with file
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Version {
    pub is_folder: bool,
    pub parent_id: String,
    pub version: String,
    pub path: String,
    pub md5: Option<String>,
}

pub type VersionsList = std::collections::HashMap<String, Version>;
/// Represents a single item in the array of versions
pub type VersionsItem = (String, Version);

pub struct Versions {
    path: PathBuf,
    is_locked: bool,
}

// TODO: Check if file is accessed by other thread before reading it to avoid errors
impl Versions {
    pub fn new(path: PathBuf) -> Result<Self> {
        // Check if file accessible
        match fs::OpenOptions::new().create(true).write(true).open(&path) {
            Ok(_) => Ok(Self {
                path,
                is_locked: false,
            }),
            Err(e) => {
                bail!("Unable to access versions file, this file is required for program to work.\nDetails: {}", e);
            }
        }
    }

    /// Finds item by path field.
    pub fn find_item_by_path(p: PathBuf, l: &VersionsList) -> Option<VersionsItem> {
        let p = p.display().to_string();
        let v = l.iter().find(|&v| v.1.path == p);
        if let Some(v) = v {
            return Some((v.0.clone(), v.1.clone()));
        }
        None
    }

    pub fn list(&mut self) -> Result<VersionsList> {
        loop {
            if self.is_locked {
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
            break;
        }

        match fs::read_to_string(&self.path) {
            Ok(content) => match serde_json::from_str::<VersionsList>(content.as_str()) {
                Ok(r) => {
                    self.is_locked = true;
                    Ok(r)
                }
                Err(_) => Ok(VersionsList::new()),
            },
            Err(e) => {
                bail!("Failed to read versions file. Details: {}", e);
            }
        }
    }

    pub fn save(&mut self, versions: VersionsList) -> Result<()> {
        match fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&self.path)
        {
            Ok(mut f) => {
                let content = serde_json::to_string(&versions)?;
                match f.write_all(content.as_bytes()) {
                    Ok(_) => {
                        self.is_locked = false;
                        Ok(())
                    }
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
