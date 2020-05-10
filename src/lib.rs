#![deny(unsafe_code)]

//! # tophat
//!
//! ## Constructing a Response:
//! - a `tophat::Response` is just an alias for an `http::Response<Body>`, where `Body` is
//! tophat-specific streaming body. There are convenience methods for constructing bodies easily
//! from streams and buffers.
//! - content-type headers must be set manually. tophat exposes the the `mime` lib for those who
//! wish to use types for MIME.
//! - body-type headers are ignored, because the encoder sets them depending on the type of body
//! set:
//!   - from streaming reader with length: fixed body
//!   - from streaming reader without length: transfer-encoding, chunked
//!   - from buffer (`Vec<u8>` or `String` or `&str`): fixed body
//! - In the future, there may be some convenience methods for constructing common responses.

mod body;
mod chunked;
mod error;
mod request;
mod response;
pub mod server;
mod timeout;
pub mod trailers;
mod util;

pub use body::Body;
pub use error::{Error, Result};
pub use request::Request;
pub use response::Response;
pub use mime;
