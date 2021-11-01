pub mod errors;
pub mod types;
use anyhow::{bail, Result};
use errors::DriveError;
use reqwest::blocking::Client as HttpClient;
use serde::{Deserialize, Serialize};
use types::{File, FileList, FileUploadBody};

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

// TODO: Cover all error cases with cases in errors enum
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

    fn get(&self, url: String, query: &[(&str, &str)]) -> Result<reqwest::blocking::Response> {
        if let Some(auth) = &self.auth {
            match self
                .http
                .get(&url)
                .bearer_auth(auth.access_token.clone())
                .header("Content-Type", "application/json")
                .query(query)
                .send()
            {
                Ok(resp) => {
                    if resp.status() == 401 {
                        bail!(DriveError::Unauthorized);
                    }
                    return Ok(resp);
                }
                Err(_) => bail!("Request failed (GET {})", url),
            }
        }

        bail!(DriveError::Unauthorized);
    }

    fn get_json<T>(&self, url: String, query: &[(&str, &str)]) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        match self.get(url, query) {
            Ok(resp) => {
                if resp.status() == 404 {
                    bail!(DriveError::NotFound);
                }
                match resp.json::<T>() {
                    Ok(data) => Ok(data),
                    Err(e) => {
                        bail!("Failed to desirialize JSON data.\nError: {}", e);
                    }
                }
            }
            Err(e) => Err(e),
        }
    }

    fn get_token(
        &self,
        refresh: bool,
        auth_code: Option<String>,
        refresh_token: Option<String>,
    ) -> Result<Session> {
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
        {
            Ok(resp) => {
                if resp.status() == 401 {
                    bail!(DriveError::Unauthorized);
                }

                match resp.json::<Session>() {
                    Ok(session) => Ok(session),
                    Err(e) => {
                        bail!("Failed to deserialize auth data.\nDetails: {}", e);
                    }
                }
            }
            Err(e) => {
                bail!("Failed to refresh access token.\nDetails: {}", e);
            }
        }
    }

    pub fn authorize_with_code(&mut self, code: String) -> Result<Session> {
        let session = self.get_token(false, Some(code.clone()), None)?;
        self.set_session(session.clone());

        Ok(session)
    }

    pub fn refresh_token(&mut self) -> Result<Session> {
        if let Some(auth) = &self.auth {
            if let Some(refresh_token) = &auth.refresh_token {
                let mut new_session = self.get_token(true, None, Some(refresh_token.clone()))?;

                new_session.refresh_token = Some(String::from(refresh_token));
                self.set_session(new_session.clone());

                return Ok(new_session);
            }
        }

        bail!("Unable to update access token since no refresh token existing");
    }

    /// Performs a GET Request to /files route, listing all files that meet `query` parameter
    /// - query is empty by default
    /// - fields are the list of all fields that are present in `File` struct
    pub fn list_files(&self, query: Option<&str>, fields: Option<&str>) -> Result<FileList> {
        self.get_json::<FileList>(
            "https://www.googleapis.com/drive/v3/files".to_string(),
            &[
                ("q", query.unwrap_or("")),
                (
                    "fields",
                    fields.unwrap_or(
                        "files(id, md5Checksum, name, trashed, mimeType, parents, version)",
                    ),
                ),
            ],
        )
    }

    pub fn get_file(&self, id: &str) -> Result<Option<File>> {
        match self.get_json(
            format!("https://www.googleapis.com/drive/v3/files/{}", id),
            &[("fields", "id, name, mimeType, parents, version")],
        ) {
            Ok(f) => Ok(Some(f)),
            Err(e) => {
                if let Some(drive_err) = e.downcast_ref::<DriveError>() {
                    return match drive_err {
                        DriveError::NotFound => Ok(None),
                        _ => bail!(e),
                    };
                }
                bail!(e);
            }
        }
    }

    /// Kind of a shortcut to `list_files` when getting the first file with some name (if it has duplicates)
    pub fn get_file_by_name(&self, name: &str, parent_id: Option<String>) -> Result<Option<File>> {
        let list = self.list_files(
            Some(&format!(
                "name = '{}' and '{}' in parents",
                &name,
                &parent_id.unwrap_or(String::from(""))
            )),
            None,
        )?;

        if list.files.len() == 0 {
            return Ok(None);
        }

        Ok(Some(list.files[0].clone()))
    }

    pub fn download_file(&self, id: &str) -> Result<Vec<u8>> {
        match self.get(
            format!("https://www.googleapis.com/drive/v3/files/{}", id),
            &[("alt", "media")],
        ) {
            Ok(resp) => Ok(resp.bytes().unwrap().into_iter().collect::<Vec<u8>>()),
            Err(e) => Err(e),
        }
    }

    pub fn create_dir(&self, name: &str, parent_id: String) -> Result<File> {
        if let Some(auth) = &self.auth {
            let body = FileUploadBody {
                name: name.to_string(),
                parents: vec![parent_id],
                mime_type: Some("application/vnd.google-apps.folder".to_string())
            };

            // Initialize uploading with sending first request in the sequence
            let res = self
                .http
                .post("https://www.googleapis.com/drive/v3/files")
                .query(&[("fields", "*")])
                .bearer_auth(auth.access_token.clone())
                .header("Content-Type", "application/json")
                .body(serde_json::to_string(&body).unwrap())
                .send()?;

            if res.status() == 401 {
                bail!(DriveError::Unauthorized);
            }

            return Ok(res.json::<File>()?);
        }

        bail!(DriveError::Unauthorized);
    }

    pub fn upload_file(
        &self,
        name: &str,
        parent_id: String,
        contents: Vec<u8>,
    ) -> Result<File> {
        if let Some(auth) = &self.auth {
            let body = FileUploadBody {
                name: name.to_string(),
                parents: vec![parent_id],
                mime_type: None 
            };

            // Initialize uploading with sending first request in the sequence
            let res = self
                .http
                .post("https://www.googleapis.com/upload/drive/v3/files")
                .bearer_auth(auth.access_token.clone())
                .header("Content-Type", "application/json")
                .query(&[("uploadType", "resumable"), ("fields", "*")])
                .body(serde_json::to_string(&body).unwrap())
                .send()?;

            if res.status() == 401 {
                bail!(DriveError::Unauthorized);
            }

            let upload_location = res.headers().get("Location");

            if upload_location.is_none() {
                bail!("Unable to create resumable session to upload file to the cloud");
            }
            let upload_location = upload_location.unwrap();

            // Upload at once, at the end we should get all file data
            let created = self
                .http
                .put(upload_location.to_str().unwrap())
                .bearer_auth(auth.access_token.clone())
                .body(contents)
                .send()?;

            if created.status() != 200 && created.status() != 201 {
                bail!("File wasn't uploaded successfully");
            }

            // TODO: Handle errors with JSON Deserialization
            return Ok(created.json::<File>()?);
        }

        bail!(DriveError::Unauthorized);
    }

    pub fn update_file(&self, id: String, contents: Vec<u8>) -> Result<File> {
        if let Some(auth) = &self.auth {
            let res = self
                .http
                .patch(format!(
                    "https://www.googleapis.com/upload/drive/v3/files/{}",
                    id
                ))
                .bearer_auth(auth.access_token.clone())
                .query(&[("uploadType", "media")])
                .body(contents)
                .send()?;

            if res.status() == 401 {
                bail!(DriveError::Unauthorized);
            }

            return Ok(res.json::<File>()?);
        }

        bail!(DriveError::Unauthorized);
    }
}
