use thiserror::Error as ThisError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(ThisError, Debug)]
pub enum Error {
    // this is error on body
    #[error("Io: {0}")]
    Io(std::io::Error),
    #[error("Http error: {0}")]
    Http(#[from] http::Error),

    #[error("Http transfer encoding not supported")]
    HttpTransferEncodingNotSupported,
}
