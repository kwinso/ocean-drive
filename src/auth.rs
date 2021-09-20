use crate::{google_drive::{Client, Session}, files, parse_url, readline::prompt, user};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Creds {
    pub client_id: String,
    pub client_secret: String,
}


pub async fn authorize() -> Result<(), String> {
    let creds = get_client_creds();
    let redirect_uri = "http://localhost:8080";
    let mut drive_client = Client::new(creds.0.clone(), creds.1.clone(), redirect_uri.to_string());

    let user_consent_url = drive_client.get_user_authorization_url("https://www.googleapis.com/auth/drive", redirect_uri);

    // TODO: Maybe it'll be greate to automatically open the browser?
    // TODO: (of course ask the permission before!)
    println!(
        "\nPlease, authorize application via this link:\n  {}\n",
        user_consent_url
    );

    // Wait for callback from user and get a requested URL
    let requested_url = crate::redirect_listener::get_callback().await.unwrap();
    let query = parse_url::get_query(urlencoding::decode(&requested_url).unwrap().to_string())
        .expect("Failed to parse request url");

    // In your redirect URL capture the code sent and our state.
    // Send it along to the request for the token.
    let code = query.get("code");

    if code.is_none() {
        return Err(
            "Unable to authorize app. Niether `code` or `state` variables was not set".to_string(),
        );
    }

    let session = drive_client.get_session_with_code(code.unwrap().to_string()).await;

    match session {
        Ok(session) => {
            println!("App is authorized. Saving user credentials and session files.");
            let creds = Creds {
                client_id: creds.0,
                client_secret: creds.1,
            };

            let home = user::get_home()?;
            let config_dir = home.join(".config/ocean-drive");

            let creds_file = config_dir.join("creds.toml");
            let session_file = config_dir.join("session.toml");
                

            files::write_toml(session, &session_file)?;
            files::write_toml(creds, &creds_file)?;
            Ok(())
        }
        Err(e) => Err(format!("Failed to authorize the app. Error: {}", e)),
    }
}

fn get_client_creds() -> (String, String) {
    let client_id = prompt("Google OAuth client id").expect("Unable to read this text");
    let client_secret = prompt("Google OAuth client secret").expect("Unable to read this text");

    return (client_id, client_secret);
}
