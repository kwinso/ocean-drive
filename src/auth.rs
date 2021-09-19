use crate::{drive::GoogleDrive, files, parse_url, readline::prompt, user};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Creds {
    pub client_id: String,
    pub client_secret: String,
}

pub async fn authorize() -> Result<(), String> {
    let creds = get_client_creds();
    let mut drive_client = GoogleDrive::get_client(creds.0.clone(), creds.1.clone(), None, None);

    // Get the URL to request consent from the user.
    // You can optionally pass in scopes. If none are provided, then the
    // resulting URL will not have any scopes.
    let mut user_consent_url =
        drive_client.user_consent_url(&["https://www.googleapis.com/auth/drive".to_string()]);

    // replace empty domain with the working one
    user_consent_url = user_consent_url.replacen(
        "https://",
        "https://accounts.google.com/o/oauth2/v2/auth",
        1,
    );
    // Option to get refresh token
    user_consent_url.push_str("&access_type=offline");

    // TODO: Maybe it'll be greate to automatically open the browser?
    // TODO: (of course ask the permission before!)
    println!(
        "\nPlease, authorize application via this link:\n  {}\n",
        user_consent_url
    );

    // Wait for callback from user and get a requested URL
    let requested_url = crate::redirect_listener::get_callback().await.unwrap();
    let query = parse_url::get_query(urldecode::decode(requested_url))
        .expect("Failed to parse request url");

    // In your redirect URL capture the code sent and our state.
    // Send it along to the request for the token.
    let code = query.get("code");
    let state = query.get("state");

    if code.is_none() || state.is_none() {
        return Err(
            "Unable to authorize app. Niether `code` or `state` variables was not set".to_string(),
        );
    }

    let access_token = drive_client
        .get_access_token(code.unwrap(), state.unwrap())
        .await;

    match access_token {
        Ok(tok) => {
            println!("App is authorized. Saving user credentials and session files.");
            let creds = Creds {
                client_id: creds.0,
                client_secret: creds.1,
            };

            let home = user::get_home()?;
            let config_dir = home.join(".config/ocean-drive");

            let creds_file = config_dir
                .join("creds.toml")
                .into_os_string()
                .into_string()
                .unwrap();
            let session_file = config_dir
                .join("session.toml")
                .into_os_string()
                .into_string()
                .unwrap();

            files::write_toml::<google_drive::AccessToken>(tok, &session_file)?;
            files::write_toml::<Creds>(creds, &creds_file)?;
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
