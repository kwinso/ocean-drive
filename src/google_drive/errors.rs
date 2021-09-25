#[derive(Debug)]
pub enum DriveClientError {
    Unauthorized,
    BadJSON,
    RequestFailed,
    NoAuthorization
}