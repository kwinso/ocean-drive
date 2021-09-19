pub fn get_home() -> Result<std::path::PathBuf, String> {
    if let Some(home) = std::env::home_dir() {
        return Ok(home);
    }

    Err("Unable to locate user home".to_string())
}