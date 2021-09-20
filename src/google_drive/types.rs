use serde::{Deserialize};

#[derive(Deserialize)]
pub struct FileList {
    pub files: Vec<File>
}

#[derive(Deserialize)]
pub struct File {
    pub id: String,
    pub name: String 
}