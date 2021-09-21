pub fn get_home() -> Result<std::path::PathBuf, ()> {
    if let Some(dir) = home::home_dir() {
        return Ok(dir);
    }

    eprintln!("Unable to locate user home directory");
    Err(())
}