use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct FileList {
    pub files: Vec<File>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct File {
    pub id: Option<String>,
    pub name: Option<String>,
    pub trashed: Option<bool>,
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
    #[serde(rename = "md5Checksum")]
    pub md5: Option<String>,
    pub version: Option<String>,
    pub parents: Option<Vec<String>>,
}
