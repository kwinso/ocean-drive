pub mod util;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Creds {
    pub client_id: String,
    pub client_secret: String,
}
