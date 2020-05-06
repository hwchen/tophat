use thiserror::Error as ThisError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(ThisError, Debug)]
pub enum Error {
    #[error("Connection error: {0}")]
    Connection(std::io::Error),

    // This needs to get manually mapped, since the automatic From is for tcp connection errors, of
    // which there are many more.
    #[error("Io error: {0}")]
    Io(std::io::Error),

    #[error("Http header parsing error: {0}")]
    HeaderParse(#[from] httparse::Error),

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

    // TODO check that these are actually errors, and not just something to handle
    #[error("Http: no version found")]
    HttpNoVersion,
    #[error("Http no path found")]
    HttpNoPath,
    #[error("Http no method found")]
    HttpNoMethod,
    #[error("Http invalid content length")]
    HttpInvalidContentLength,
}
