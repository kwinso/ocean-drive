mod types;
use types::{FileList};
use crate::{
    auth::Creds,
    files,
    requests,
    // setup::Config,
    user,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Session {
    pub access_token: String,
    pub refresh_token: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct Config {
    pub dir: String,
}

pub struct Client {
    client_id: String,
    client_secret: String,
    redirect_uri: String,
    auth: Option<Session>,
}


impl Client {
    pub fn new(client_id: String, client_secret: String, redirect_uri: String) -> Self {
        Self {
            client_id,
            client_secret,
            redirect_uri,
            auth: None,
        }
    }

    pub fn get_user_authorization_url(&self, scope: &str, redirect_uri: &str) -> String {
        format!(
            "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&response_type=code&redirect_uri={}&scope={}&access_type=offline",
            urlencoding::encode(&self.client_id),
            urlencoding::encode(redirect_uri),
            urlencoding::encode(scope)
        )
    }

    pub async fn get_session_with_code(&mut self, code: String) -> Result<Session, String> {
        let session_data = format!(
            "client_id={}&client_secret={}&redirect_uri={}&code={}&grant_type=authorization_code",
            urlencoding::encode(&self.client_id), 
            urlencoding::encode(&self.client_secret), 
            urlencoding::encode(&self.redirect_uri), 
            urlencoding::encode(&code)
        );
        let session = requests::post(
            "https://oauth2.googleapis.com/token".to_string(),
            session_data,
            vec![("Content-Type", "application/x-www-form-urlencoded")]
        )
        .await?;

        Ok(session)
    }

    pub fn set_session(&mut self, s: Session) {
        self.auth = Some(s);
    }

    pub async fn get_new_session(&mut self, refresh_token: String) -> Result<Option<Session>, String> {
        if let Some(_) = &self.auth {
            let refresh_token_request = format!(
                "client_id={}&client_secret={}&grant_type=refresh_token&refresh_token={}",
                urlencoding::encode(&self.client_id),
                urlencoding::encode(&self.client_secret),
                urlencoding::encode(&refresh_token)
            );
            let new_session = requests::post::<Session>(
                "https://oauth2.googleapis.com/token".to_string(),
                refresh_token_request,
                vec![("Content-Type", "application/x-www-form-urlencoded")],
            )
            .await?;


            return Ok(Some(Session {
                access_token: new_session.access_token,
                refresh_token: Some(refresh_token),
            }));
        }

        Ok(None)
    }

    pub async fn list_files(&self, query: Option<&str>) -> Result<FileList, String> {
        unimplemented!();

        // TODO
        // let request_uri = format!("https://www.googleapis.com/drive/v3/files?q={}", query.unwrap_or(""));
        // requests::get::<FileList>(
        //     request_uri,
        //     vec![("Authorization", &self.auth.unwrap().access_token.clone()]
        // ).await
    }
    // pub async fn query_files(&self, query: &str) -> Result<Vec<File>, String> {
    //     match (&self)
    //         .default
    //         .files()
    //         .list(
    //             "", "", false, "", false, "", 10, "", query, "", false, false, "",
    //         )
    //         .await
    //     {
    //         Ok(list) => Ok(list),
    //         Err(e) => Err(format!("Failed to get files in drive: {}", e)),
    //     }
}

//     pub async fn get_file(&self, file_id: &str) -> Result<File, String> {
//         match (&self)
//             .default
//             .files()
//             .get(file_id, false, "", false, false)
//             .await
//         {
//             Ok(f) => Ok(f),
//             Err(e) => Err(format!("Unable to get file with id '{}': {}", e, file_id)),
//         }
//     }
// }

// pub struct GoogleDrive {
//     pub client: Client,
//     pub config: DriveConfig,
// }

// impl GoogleDrive {
//     pub async fn new() -> Result<Self, String> {
//         let conf_dir = user::get_home()?.join(".config/ocean-drive");
//         let creds_file = conf_dir.join("creds.toml");
//         let session_file = conf_dir.join("session.toml");
//         let conf_file = conf_dir.join("config.toml");

//         let session = files::read_toml::<AccessToken>(&session_file)?;
//         let creds = files::read_toml::<Creds>(&creds_file)?;
//         let conf = files::read_toml::<Config>(&conf_file)?;

//         let mut client = GoogleDrive::get_client(
//             creds.client_id.clone(),
//             creds.client_secret.clone(),
//             Some(session.access_token),
//             Some(session.refresh_token.clone()),
//         );

//         if let Ok(new_session) = client.refresh_access_token().await {
//             // Since refresh token is not updated, we keep it as it was before
//             let mut new_session = new_session;
//             new_session.refresh_token = session.refresh_token;

//             files::write_toml(Session::from_api_response(new_session), &session_file)?;
//         }

//         Ok(Self {
//             config: conf.drive,
//             // Wrap the default client to get more extendet functionality
//             client: Client { default: client },
//         })
//     }

//     // Used to get the client just for using it, excluding additional features of this struct
//     pub fn get_client(
//         client_id: String,
//         client_secret: String,
//         access_token: Option<String>,
//         refresh_token: Option<String>,
//     ) -> DefaultClient {
//         DefaultClient::new(
//             client_id,
//             client_secret,
//             String::from("http://localhost:8080"),
//             access_token.unwrap_or("".to_string()), // empty access token
//             refresh_token.unwrap_or("".to_string()), // empty refresh token
//         )
//     }
// }
