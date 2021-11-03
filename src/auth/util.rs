use crate::{
    files,
    google_drive::{Client, Session},
};
use anyhow::{bail, Result};
use std::path::Path;
use std::sync::MutexGuard;


pub fn update_for_shared_client(client: &mut MutexGuard<Client>) -> Result<()> {
    match client.refresh_token() {
       Ok(s) => {
           files::write_toml::<Session>(s, Path::new("~/.config/ocean-drive/session.toml").to_path_buf())?;
           Ok(())
       },
       Err(e) => bail!("Unable to update client authorization tokens.\nTip: try to manually run `ocean-drive auth`.\nDetails: {}", e)
    }
}
