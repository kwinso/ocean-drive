use std::fs;
use std::io::prelude::*;
use std::path::{Path};
use std::process::exit;
use toml;


pub fn read_toml<'a, T>(path: &str) -> T
where
    T: serde::de::DeserializeOwned
{
    if !Path::new(path).exists() {
        eprintln!("File does not exist: {}", path);
        exit(1);
    }

    let contents = fs::read_to_string(path).unwrap();
    match toml::from_str(&contents) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Unable to load the config: {}", e);
            exit(1);
        }
    }
}

pub fn write_toml<T>(data: T, path: &str) 
where 
    T: serde::ser::Serialize
{

    let mut file = 
        (if Path::new(path).exists() { 
            fs::File::open(path) 
        } else { 
            fs::File::create(path) 
        }).expect(&format!("Failed to open a file `{}`", path));

    let toml = toml::to_string(&data).unwrap();

    match file.write_all(toml.as_bytes()) {
        Err(e) => {
            eprintln!("Failed to save data to file `{}`. Error:\n{}", path, e);

        },
        _ => {}
    };
}