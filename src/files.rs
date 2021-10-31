use anyhow::{bail, Result};
use std::io::BufReader;
use std::io::Read;
use std::{fs, io::prelude::*, path::PathBuf};
use toml;

pub fn read_bytes(path: PathBuf) -> Result<Vec<u8>> {
    let f = fs::OpenOptions::new().write(true).read(true).open(path)?;
    let mut reader = BufReader::new(f);
    let mut buffer = Vec::new();

    // Read file into vector.
    reader.read_to_end(&mut buffer)?;

    Ok(buffer)
}

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
                    path.display(),
                    e
                ),
                _ => {}
            }
            let r = toml::from_str::<T>(&contents)?;

            Ok(r)
        }
        Err(e) => bail!(
            "Failed to open file. (File: '{}')\nDetails: {}",
            path.display(),
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
                    path.display(),
                    e
                );
            }

            Ok(())
        }
        Err(e) => bail!(
            "Failed to open file. (File: '{}')\nDetails: {}",
            path.display(),
            e
        ),
    }
}
