use google_drive::{Client, AccessToken};
use crate::{
    setup::DriveConfig,
    auth::Creds,
    files,
    user
};

pub struct GoogleDrive {
    pub client: Client,
}

impl GoogleDrive {
    pub async fn new() -> Result<Self, String> {
        let conf_dir = user::get_home()?.join(".config/ocean-drive");
        let creds_file = conf_dir.join("creds.toml").into_os_string().into_string().unwrap();
        let session_file = conf_dir.join("session.toml").into_os_string().into_string().unwrap();

        let creds = files::read_toml::<Creds>(&creds_file)?;
        let session = files::read_toml::<AccessToken>(&session_file)?;
        let mut client = GoogleDrive::get_client(creds.client_id.clone(), creds.client_secret.clone(), Some(session.access_token), Some(session.refresh_token));

        if let Ok(new_tok) = client.refresh_access_token().await { 
            files::write_toml(new_tok, &session_file)?;
        }

        Ok(Self{ client })
    }

    // Used to get the client just for using it, excluding additional features of this struct
    pub fn get_client(client_id: String, client_secret: String, access_token: Option<String>, refresh_token: Option<String>) -> Client {
        Client::new(
            client_id,
            client_secret,
            String::from("http://localhost:8080"),
            access_token.unwrap_or("".to_string()), // empty access token
            refresh_token.unwrap_or("".to_string()), // empty refresh token
        )
    }
}
