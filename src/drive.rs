use serde::Deserialize;

#[derive(Deserialize, Default)]
struct Creds {
    client_id: String,
    client_secret: String,
}

pub struct GoogleDrive {
    creds: Creds,
}

// TODO: Authorize with google
impl GoogleDrive {
    pub fn new() -> Self {
        Self {
            creds: Default::default(),
        }
    }
}
