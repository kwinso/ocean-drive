use crate::{files, google_drive::Client, parse_url, readline::prompt, redirect_listener, user};
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

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
    // TODO: Maybe it'll be greate to automatically open the browser?
    // TODO: (of course ask the permission before!)
    println!(
        "\nPlease, authorize application via this link:\n  {}\n",
        user_consent_url
    );

    let url = redirect_listener::get_callback()?;
    let query = parse_url::get_query(url)?;
    let code = query.get("code");

    if let Some(code) = code {
        return Ok(code.to_string());
    }

    bail!("There was no authorization code in Google API Callback");
}

fn get_client_creds() -> (String, String) {
    let client_id = prompt("Google OAuth client id").expect("Unable to read this text");
    let client_secret = prompt("Google OAuth client secret").expect("Unable to read this text");

    return (client_id, client_secret);
}
