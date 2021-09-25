pub mod errors;
pub mod types;
use bytes;
use errors::DriveClientError;
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};
use types::FileList;

#[derive(Serialize, Deserialize, Clone)]
pub struct Session {
    pub access_token: String,
    pub refresh_token: Option<String>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Config {
    pub dir: String,
}

#[derive(Clone)]
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

    async fn get(
        &self,
        url: &str,
        query: &[(&str, &str)],
    ) -> Result<reqwest::Response, DriveClientError> {
        if let Some(auth) = &self.auth {
            match self
                .http
                .get(url)
                .bearer_auth(auth.access_token.clone())
                .header("Content-Type", "application/json")
                .query(query)
                .send()
                .await
            {
                Ok(resp) => {
                    if resp.status() == 401 {
                        return Err(DriveClientError::Unauthorized);
                    }
                    return Ok(resp);
                }
                Err(_) => return Err(DriveClientError::RequestFailed),
            }
        }

        eprintln!("Unable to request data since no authorization set");
        Err(DriveClientError::NoAuthorization)
    }

    async fn get_json<T>(&self, url: &str, query: &[(&str, &str)]) -> Result<T, DriveClientError>
    where
        T: serde::de::DeserializeOwned,
    {
        match self.get(url, query).await {
            Ok(resp) => match resp.json::<T>().await {
                Ok(data) => Ok(data),
                Err(e) => {
                    eprintln!("Failed to desirialize JSON data.\nError: {}", e);
                    return Err(DriveClientError::BadJSON);
                }
            },
            Err(e) => Err(e),
        }
    }

    async fn get_token(
        &self,
        refresh: bool,
        auth_code: Option<String>,
        refresh_token: Option<String>,
    ) -> Result<Session, DriveClientError> {
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
            Ok(resp) => {
                if resp.status() == 401 {
                    return Err(DriveClientError::Unauthorized);
                }
                match resp.json::<Session>().await {
                    Ok(session) => Ok(session),
                    Err(e) => {
                        println!("Failed to deserialize auth data.\nError: {}", e);
                        Err(DriveClientError::BadJSON)
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to refresh access token.\nError: {}", e);
                Err(DriveClientError::RequestFailed)
            }
        }
    }

    pub async fn authorize_with_code(&mut self, code: String) -> Result<Session, DriveClientError> {
        let session = self.get_token(false, Some(code.clone()), None).await?;
        self.set_session(session.clone());

        Ok(session)
    }

    pub async fn refresh_token(&mut self) -> Result<Session, DriveClientError> {
        if let Some(auth) = &self.auth {
            if let Some(refresh_token) = &auth.refresh_token {
                let mut new_session = self
                    .get_token(true, None, Some(refresh_token.clone()))
                    .await?;

                new_session.refresh_token = Some(String::from(refresh_token));
                self.set_session(new_session.clone());

                return Ok(new_session);
            }
        }

        println!("[WARN] Unable to update access token since no refresh token existing");
        Err(DriveClientError::NoAuthorization)
    }

    pub async fn list_files(
        &self,
        query: Option<&str>,
        fields: Option<&str>,
    ) -> Result<FileList, DriveClientError> {
        let list = self
            .get_json::<FileList>(
                "https://www.googleapis.com/drive/v3/files",
                &[
                    ("q", query.unwrap_or("")),
                    ("fields", fields.unwrap_or("*")),
                ],
            )
            .await?;

        Ok(list)
    }

    pub async fn download_file(&self, id: String) -> Result<bytes::Bytes, DriveClientError> {
        match self
            .get(
                &format!("https://www.googleapis.com/drive/v3/files/{}", id),
                &[("alt", "media")],
            )
            .await {
                Ok(resp) => Ok(resp.bytes().await.unwrap()),
                Err(e) => Err(e)
            }
    }
}
