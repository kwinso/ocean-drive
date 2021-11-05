use thiserror::Error;

#[derive(Error, Debug)]
pub enum DaemonError {
    #[error("Critical Error that is unable to handle")]
    Critical(e),
    #[error("Error Successfully handled, but thrown in case ")]
    Handled,
}


