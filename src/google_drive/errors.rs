use thiserror::Error;

#[derive(Error, Debug)]
pub enum DriveError {
    #[error("Request to the API failed with status 401")]
    Unauthorized,
}
