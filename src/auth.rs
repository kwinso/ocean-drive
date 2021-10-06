use crate::{
    files,
    google_drive::{Client, Session},
    parse_url,
    readline::{binary_prompt, prompt},
    redirect_listener, user,
};
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::MutexGuard;
use webbrowser;

#[derive(Serialize, Deserialize)]
pub struct Creds {
    pub client_id: String,
    pub client_secret: String,
}

pub fn authorize() -> Result<()> {
    let creds = get_client_creds();
    let redirect_uri = "http://localhost:8080";
    let mut drive_client = Client::new(creds.0.clone(), creds.1.clone(), redirect_uri.to_string());

    let user_consent_url = drive_client
        .get_user_authorization_url("https://www.googleapis.com/auth/drive", redirect_uri);

    let auth_code = get_auth_code(user_consent_url);

    if let Ok(code) = auth_code {
        let session = drive_client.authorize_with_code(code.to_string())?;

        println!("App is authorized. Saving user credentials and session files.");

        let creds = Creds {
            client_id: creds.0,
            client_secret: creds.1,
        };

        let home = user::get_home();
        if let Ok(home) = home {
            let config_dir = home.join(".config/ocean-drive");

            let creds_file = config_dir.join("creds.toml");
            let session_file = config_dir.join("session.toml");

            files::write_toml(session, session_file)?;
            files::write_toml(creds, creds_file)?;

            return Ok(());
        }

        bail!("Failed to save user credentials.");
    }

    bail!("Failed to get authroization code from the Google API");
}

fn get_auth_code(user_consent_url: String) -> Result<String> {
    let auto_open =
        binary_prompt("Do you want to automatically open authorization url in your browser?");
    let mut successfully_opened = false;

    if auto_open {
        println!("Openning URL...");
        if webbrowser::open(&user_consent_url).is_err() {
            println!("Warn: Unable to open URL in web browser. Please, open in manually.");
        }
        successfully_opened = true;
    }

    if !successfully_opened {
        println!(
            "\nPlease, authorize application via this link:\n  {}\n",
            user_consent_url
        );
    }

    let url = redirect_listener::get_callback()?;
    let query = parse_url::get_query(url)?;
    let code = query.get("code");

    if let Some(code) = code {
        return Ok(code.to_string());
    }

    bail!("There was no authorization code in Google API Callback");
}

fn get_client_creds() -> (String, String) {
    let empty = String::from("");

    let client_id = prompt("Google OAuth client id").unwrap_or(empty.clone());
    let client_secret = prompt("Google OAuth client secret").unwrap_or(empty.clone());

    return (client_id, client_secret);
}

pub fn update_for_shared_client(client: &mut MutexGuard<Client>) -> Result<()> {
    match client.refresh_token() {
       Ok(s) => {
           files::write_toml::<Session>(s, Path::new("~/.config/ocean-drive/session.toml").to_path_buf())?;
           Ok(())
       },
       Err(e) => bail!("Unable to update client authorization tokens.\nTip: try to manually run `ocean-drive auth`.\nDetails: {}", e)
    }
}
