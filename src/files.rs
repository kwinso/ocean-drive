use std::fs;
use std::io::prelude::*;
use std::path::Path;
use toml;

pub fn read_toml<'a, T>(path: &str) -> Result<T, String>
where
    T: serde::de::DeserializeOwned,
{
    if !Path::new(path).exists() {
        return Err(format!("File does not exist: {}", path));
    }

    println!("{}", &path);

    let contents = fs::read_to_string(path).unwrap();

    println!("{}", &contents);

    match toml::from_str(&contents) {
        Ok(r) => r,
        Err(e) => Err(format!("Unable to load the config: {:#?}", e))
    }
}

pub fn write_toml<T>(data: T, path: &str) -> Result<(), String>
where
    T: serde::ser::Serialize,
{
    if Path::new(path).exists() {
        fs::remove_file(path).unwrap();
    }
    let mut file = fs::File::create(path).unwrap(); 

    let toml = toml::to_string(&data).unwrap();

    if let Err(e) = file.write_all(toml.as_bytes()) {
        return Err(format!("Failed to write data to file `{}`. Error:\n{:?}", path, e));
    }

    Ok(())
}