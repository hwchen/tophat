use thiserror::Error as ThisError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(ThisError, Debug)]
pub enum Error {
    #[error("Connection Lost: {0}")]
    Connection(std::io::Error),
    #[error("Io: {0}")]
    Io(std::io::Error),
    #[error("Http error: {0}")]
    Http(#[from] http::Error),
    #[error("Http Uri error: {0}")]
    HttpUri(#[from] http::uri::InvalidUri),
    #[error("Http Method error: {0}")]
    HttpMethod(#[from] http::method::InvalidMethod),
    #[error("Http Header name error: {0}")]
    HttpHeaderName(#[from] http::header::InvalidHeaderName),
    #[error("Http Header value error: {0}")]
    HttpHeaderValue(#[from] http::header::InvalidHeaderValue),

    #[error("Http transfer encoding not supported")]
    HttpTransferEncodingNotSupported,
}
