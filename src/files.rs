use std::{
    fs,
    io::prelude::*,
    path::{Path, PathBuf},
};
use toml;

// Todo: use OpenOptions to open files (more convinient)
pub fn read_toml<'a, T>(path: &PathBuf) -> Result<T, ()>
where
    T: serde::de::DeserializeOwned,
{
    let path_str = path.clone().into_os_string().into_string().unwrap();

    if !Path::new(path).exists() {
        eprintln!(
            "Cannot read the file because it does not exists: {}",
            path_str
        );
        return Err(());
    }

    let contents = fs::read_to_string(path).unwrap();
    match toml::from_str::<T>(&contents) {
        Ok(r) => Ok(r),
        Err(e) => {
            eprintln!(
                "Unable to process file contents in file '{}'.\nError: {}",
                path.clone()
                    .into_os_string()
                    .to_str()
                    .unwrap_or("<failed to show filename>"),
                e
            );
            Err(())
        }
    }
}

pub fn write_toml<T>(data: T, path: &PathBuf) -> Result<(), ()>
where
    T: serde::ser::Serialize,
{
    if Path::new(&path).exists() {
        fs::remove_file(path).unwrap();
    }
    let mut file = fs::File::create(path).unwrap();

    let toml = toml::to_string(&data).unwrap();

    if let Err(e) = file.write_all(toml.as_bytes()) {
        eprintln!(
            "Failed to write contents to file `{}`.\nError: {}",
            path.clone()
            .into_os_string()
            .to_str()
            .unwrap_or("<failed to show filename>"),
            e
        );
        return Err(());
    }

    Ok(())
}
