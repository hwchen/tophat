//! Response type for handling Endpoint errors.
//!
//! ## Overview
//! Errors returned by users within an Endpoint are never meant to be bubbled up all the way to the
//! server to handle; instead, they should be caught immediately after the endpoint, where they are
//! transformed into a Response.
//!
//! This means that a Glitch is not an Error in the traditional Rust sense, it's very
//! Response-specific.
//!
//! Without this functionality, a user will always have to create their own error responses and
//! manually return them, without the convenience of Rust's built-in `Result` and `?` operator.
//!
//! ## Functionality
//! A `Glitch` allows you to:
//! - Just use `?` on any error, and it will be turned into a 500 response. (`anyhow` feature
//! only)
//! - use `.map_err` to easily convert your error to a Glitch.
//!
//! In this system, it's easy to use standard `From` and `Into` traits to convert your custom
//! errors if you want.
// I think that this is unlike warp, which requires you to match on your error in a `catch`, and
// then convert your error to a response then? Here, your error is converted on the spot.
//!

use http::{
    header::HeaderMap,
    status::StatusCode,
    version::Version,
};

use crate::server::InnerResponse;

pub type Result<T> = std::result::Result<T, Glitch>;

// similar to inner_response, but with string-only body
#[derive(Debug)]
pub struct Glitch {
    pub(crate) status: Option<StatusCode>,
    pub(crate) headers: Option<HeaderMap>,
    pub(crate) version: Option<Version>,
    pub(crate) message: Option<String>,

    // This is an Option in case somebody has anyhow feature chosen, but just wants to
    // directly make a Glitch without converting from an error using `From`.
    #[cfg(feature = "anyhow")]
    pub(crate) anyhow: Option<anyhow_1::Error>,
}

#[cfg(feature = "anyhow")]
impl<E> From<E> for Glitch
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn from(error: E) -> Self {
        Self::new_with_anyhow(error)
    }
}

impl Glitch {
    #[allow(dead_code)] // only used by Cors so far
    pub(crate) fn new() -> Self {
        Self {
            status: None,
            headers: None,
            version: None,
            message: None,
            #[cfg(feature = "anyhow")]
            anyhow: None,
        }
    }

    #[cfg(feature = "anyhow")]
    pub(crate) fn new_with_anyhow<E>(error: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self {
            status: None,
            headers: None,
            version: None,
            message: None,
            anyhow: Some(error.into()),
        }
    }

    pub(crate) fn into_inner_response(self) -> InnerResponse {
        // TODO only return anyhow error in body if some debug flag is turned on
        //#[cfg(feature = "anyhow")]
        //let body = if let Some(message) = self.message {
        //    message.into()
        //} else if let Some(any_err) = self.anyhow {
        //    any_err.to_string().into()
        //} else {
        //    Body::empty()
        //};

        let body =  self.message.unwrap_or_else(|| "".to_string()).into();

        InnerResponse {
            status: self.status.unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            headers: self.headers.unwrap_or_else(HeaderMap::new),
            version: self.version.unwrap_or(Version::HTTP_11),
            body,
        }
    }

    pub fn bad_request() -> Self {
        Self {
            status: Some(StatusCode::BAD_REQUEST),
            headers: None,
            version: None,
            message: None,
            #[cfg(feature = "anyhow")]
            anyhow: None,
        }
    }

    pub fn internal_server_error() -> Self {
        Self {
            status: None,
            headers: None,
            version: None,
            message: None,
            #[cfg(feature = "anyhow")]
            anyhow: None,
        }
    }
}
