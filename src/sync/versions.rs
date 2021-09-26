/*
    This struct is used to manage versions.json file
    It can be shared between threads and used with mutex to avoid threads trying to read/write to file simulteniosly
*/
use serde::{Deserialize, Serialize};
use serde_json;
use std::fs;
use std::io;
use std::io::Write;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone)]
pub struct VersionLog {
    pub version: String,
    // pub local_hash: String,
    // pub remote_hash: String,
}

pub type VersionsList = std::collections::HashMap<String, VersionLog>;

pub struct Versions {
    path: PathBuf,
}

impl Versions {
    pub fn new(path: PathBuf) -> Result<Self, ()> {
        let file: io::Result<fs::File>;

        if !path.exists() {
            file = fs::File::create(&path);
        } else {
            file = fs::File::create(&path)
        }

        match file {
            Ok(_) => Ok(Self { path }),
            Err(e) => {
                eprintln!("Unable to create versions file, this file is required for program to work.\nError: {}", e);
                Err(())
            }
        }
    }

    pub fn get_all(&self) -> Result<VersionsList, ()> {
        match fs::read_to_string(&self.path) {
            Ok(content) => match serde_json::from_str::<VersionsList>(content.as_str()) {
                Ok(r) => Ok(r),
                Err(_) => Ok(VersionsList::new()),
            },
            Err(e) => {
                eprintln!("Failed to read versions file. Error: {}", e);
                Err(())
            }
        }
    }

    pub fn get(&self, id: String) -> Result<Option<VersionLog>, ()> {
        let versions = self.get_all()?;

        if versions.contains_key(&id) {
            let entry = versions.get(&id).unwrap().clone();
            return Ok(Some(entry));
        }

        Ok(None)
    }

    pub fn set(&self, id: String, log: VersionLog) -> Result<(), ()> {
        let mut versions = self.get_all()?;

        versions.entry(id).or_insert(log);

        match fs::OpenOptions::new()
            .write(true)
            .append(false)
            .open(&self.path)
        {
            Ok(mut f) => {
                let content = serde_json::to_string(&versions).unwrap();
                match f.write_all(content.as_bytes()) {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        eprintln!("Failed to save versions data to file.\nError: {}", e);
                        Err(())
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to open versions file.\nError: {}", e);
                Err(())
            }
        }
    }
}
