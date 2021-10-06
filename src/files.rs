use anyhow::{bail, Result};
use std::{fs, io::prelude::*, path::PathBuf};
use toml;

pub fn read_toml<'a, T>(path: PathBuf) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    match fs::OpenOptions::new().read(true).open(&path) {
        Ok(mut f) => {
            let mut contents = String::new();
            match f.read_to_string(&mut contents) {
                Err(e) => bail!(
                    "Unable to read file contents. (File: '{}')\nDetails: {}",
                    path.into_os_string().into_string().unwrap(),
                    e
                ),
                _ => {}
            }
            let r = toml::from_str::<T>(&contents)?;

            Ok(r)
        }
        Err(e) => bail!(
            "Failed to open file. (File: '{}')\nDetails: {}",
            path.into_os_string().into_string().unwrap(),
            e
        ),
    }
}

pub fn write_toml<T>(data: T, path: PathBuf) -> Result<()>
where
    T: serde::ser::Serialize,
{
    match fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&path)
    {
        Ok(mut f) => {
            let toml = toml::to_string(&data).unwrap();

            if let Err(e) = f.write_all(toml.as_bytes()) {
                bail!(
                    "Failed to write contents to file `{}`.\nDetails: {}",
                    path.into_os_string().into_string().unwrap(),
                    e
                );
            }

            Ok(())
        }
        Err(e) => bail!(
            "Failed to open file. (File: '{}')\nDetails: {}",
            path.into_os_string().into_string().unwrap(),
            e
        ),
    }
}
