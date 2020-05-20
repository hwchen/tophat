#![deny(unsafe_code)]
#![warn(missing_docs)]

//! # tophat
//!
//! A small, pragmatic, and flexible async HTTP server library.
//!
//! More docs coming soon! For now, please see the examples directory for features.
//!
//! Also, please note that you'll need to set up your own async runtime to work with tophat. All
//! the examples use `smol` as the runtime.

mod body;
mod chunked;
mod error;
mod request;
mod response;
pub mod server;
mod timeout;
pub mod trailers;
mod util;

/// Re-export http crate for convenience
pub use http;

pub use body::Body;
pub use error::Error;
pub use request::Request;
pub use response::Response;
