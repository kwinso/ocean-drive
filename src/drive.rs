use crate::config::DriveConfig;
use google_drive::Client;
use crate::auth::Creds;
use crate::files;


pub struct GoogleDrive {
    creds: Creds,
    client: Client,
}

impl GoogleDrive {
    pub fn new(config: DriveConfig) -> Self {
        let creds = files::read_toml::<Creds>("./creds.toml");
        let client = GoogleDrive::get_client(creds.client_id.clone(), creds.client_secret.clone());

        Self{ creds: creds, client }
    }

    // Used to get the client just for using it, excluding additional features of this struct
    pub fn get_client(client_id: String, client_secret: String) -> Client {
        Client::new(
            client_id,
            client_secret,
            String::from("http://localhost:8080"),
            String::from(""), // empty access token
            String::from(""), // empty refresh token
        )
    }
}
