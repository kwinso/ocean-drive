mod types;
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};
use types::FileList;

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
    http: HttpClient,
}

impl Client {
    pub fn new(client_id: String, client_secret: String, redirect_uri: String) -> Self {
        Self {
            client_id,
            client_secret,
            redirect_uri,
            auth: None,
            http: HttpClient::new(),
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

    pub fn set_session(&mut self, s: Session) {
        self.auth = Some(s);
    }

    async fn get_token(
        &self,
        refresh: bool,
        auth_code: Option<String>,
        refresh_token: Option<String>,
    ) -> Session {
        let mut params: Vec<(&str, String)> = vec![
            ("client_id", self.client_id.clone()),
            ("client_secret", self.client_secret.clone()),
            ("redirect_uri", self.redirect_uri.clone()),
        ];

        if refresh {
            params.push(("refresh_token", refresh_token.unwrap_or(String::from(""))));
            params.push(("grant_type", String::from("refresh_token")));
        } else {
            params.push(("code", auth_code.unwrap_or(String::from(""))));
            params.push(("grant_type", String::from("authorization_code")));
        }

        match self
            .http
            .post("https://oauth2.googleapis.com/token")
            .form(&params)
            .send()
            .await
        {
            Ok(resp) => match resp.json::<Session>().await {
                Ok(session) => session,
                Err(e) => panic!("Failed to deserialize auth data.\nError: {}", e),
            },
            Err(e) => panic!("Failed to refresh access token.\nError: {}", e),
        }
    }

    pub async fn authorize_with_code(&mut self, code: String) -> Session {
        let session = self.get_token(false, Some(code.clone()), None).await;
        self.set_session(session.clone());

        session
    }

    pub async fn update_auth(&mut self, refresh_token: String) -> Session {
        let mut new_session = self.get_token(true, None, Some(refresh_token.clone())).await;

        new_session.refresh_token = Some(String::from(refresh_token));
        self.set_session(new_session.clone());

        new_session
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
}