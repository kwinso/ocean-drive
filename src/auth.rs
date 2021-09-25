use crate::{
    files,
    google_drive::{errors::DriveClientError, Client},
    parse_url,
    readline::prompt,
    redirect_listener, user,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Creds {
    pub client_id: String,
    pub client_secret: String,
}

pub async fn authorize() -> Result<(), ()> {
    let creds = get_client_creds();
    let redirect_uri = "http://localhost:8080";
    let mut drive_client = Client::new(creds.0.clone(), creds.1.clone(), redirect_uri.to_string());

    let user_consent_url = drive_client
        .get_user_authorization_url("https://www.googleapis.com/auth/drive", redirect_uri);

    let auth_code = get_auth_code(user_consent_url).await;

    if let Ok(code) = auth_code {
        match drive_client.authorize_with_code(code.to_string()).await {
            Ok(session) => {
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

                    files::write_toml(session, &session_file)?;
                    files::write_toml(creds, &creds_file)?;

                    return Ok(());
                }

                return Err(());
            }
            Err(e) => {
                match e {
                    DriveClientError::Unauthorized => {
                        eprintln!("Bad authorization code. Responded with 401");
                    }
                    _ => {}
                }

                return Err(());
            }
        }
    }

    Err(())
}

async fn get_auth_code(user_consent_url: String) -> Result<String, ()> {
    // TODO: Maybe it'll be greate to automatically open the browser?
    // TODO: (of course ask the permission before!)
    println!(
        "\nPlease, authorize application via this link:\n  {}\n",
        user_consent_url
    );

    match redirect_listener::get_callback().await {
        Ok(url) => {
            match parse_url::get_query(urlencoding::decode(&url).unwrap().to_string()) {
                Ok(query) => {
                    // In your redirect URL capture the code sent and our state.
                    // Send it along to the request for the token.
                    let code = query.get("code");

                    if code.is_none() {
                        eprintln!("Unable to get authorization code from Google.");
                        return Err(());
                    }

                    Ok(code.unwrap().to_string())
                }
                Err(e) => {
                    eprintln!("Error while getting authorization code:\n{}", e);
                    Err(())
                }
            }
        }
        Err(_) => Err(()),
    }
}

fn get_client_creds() -> (String, String) {
    let client_id = prompt("Google OAuth client id").expect("Unable to read this text");
    let client_secret = prompt("Google OAuth client secret").expect("Unable to read this text");

    return (client_id, client_secret);
}
