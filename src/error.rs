use thiserror::Error as ThisError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(ThisError, Debug)]
pub enum Error {
    #[error("tcp connection error:{0}")]
    TcpConnection(#[from] std::io::Error),
    #[error("Http header parsing error:{0}")]
    HeaderParse(#[from] httparse::Error),
    #[error("Http error:{0}")]
    Http(#[from] http::Error),
}
