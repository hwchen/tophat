use thiserror::Error as ThisError;

pub type Result<T> = std::result::Result<T, Error>;

/// Public Errors (does not include internal fails)
#[derive(ThisError, Debug)]
pub enum Error {
    #[error("Error sending response: {0}")]
    ResponseSend(std::io::Error),
    // this is error on body
    #[error("Error converting body: {0}")]
    BodyConversion(std::io::Error),
}
