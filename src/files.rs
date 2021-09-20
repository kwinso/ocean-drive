use std::{
    fs,
    io::prelude::*,
    path::{Path, PathBuf}

};
use toml;

pub fn read_toml<'a, T>(path: &PathBuf) -> Result<T, String>
where
    T: serde::de::DeserializeOwned,
{
    let path_str = path.clone().into_os_string().into_string().unwrap();

    if !Path::new(path).exists() {
        return Err(format!("Cannot read the file because it does not exists: {}", path_str));
    }

    let contents = fs::read_to_string(path).unwrap();
    match toml::from_str::<T>(&contents) {
        Ok(r) => Ok(r),
        Err(e) => Err(format!("Unable to load the config: {:#?}", e))
    }
}

pub fn write_toml<T>(data: T, path: &PathBuf) -> Result<(), String>
where
    T: serde::ser::Serialize,
{
    if Path::new(&path).exists() {
        fs::remove_file(path).unwrap();
    }
    let mut file = fs::File::create(path).unwrap(); 

    let toml = toml::to_string(&data).unwrap();

    if let Err(e) = file.write_all(toml.as_bytes()) {
        return Err(format!("Failed to write data to file `{}`. Error:\n{:?}", path.clone().into_os_string().into_string().unwrap(), e));
    }

    Ok(())
}