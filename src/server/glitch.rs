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
use std::convert::Infallible;
use std::fmt::Display;

use crate::server::InnerResponse;

pub type Result<T> = std::result::Result<T, Glitch>;

// similar to inner_response, but with string-only body
#[derive(Debug)]
pub struct Glitch {
    pub(crate) status: Option<StatusCode>,
    pub(crate) headers: Option<HeaderMap>,
    pub(crate) version: Option<Version>,
    pub(crate) message: Option<String>,

    // keep things simple, this is just response so no need to hold an actual error. Just print the
    // error string.
    pub(crate) trace: Option<String>,
}

impl<E> From<E> for Glitch
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn from(error: E) -> Self {
        Self::new_with_err(error)
    }
}

impl std::default::Default for Glitch {
    fn default() -> Self {
        Self {
            status: None,
            headers: None,
            version: None,
            message: None,
            trace: None,
        }
    }
}

impl Glitch {
    pub fn new() -> Self {
        Self {
            status: None,
            headers: None,
            version: None,
            message: None,
            trace: None,
        }
    }

    pub(crate) fn new_with_err<E>(error: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self {
            status: None,
            headers: None,
            version: None,
            message: None,
            trace: Some(error.to_string()),
        }
    }

    pub(crate) fn new_with_context<C>(context: C) -> Self
    where
        C: Display + Send + Sync + 'static,
    {
        Self {
            status: None,
            headers: None,
            version: None,
            message: Some(context.to_string()),
            trace: None,
        }
    }

    pub(crate) fn new_with_err_context<E, C>(error: E, context: C) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
        C: Display + Send + Sync + 'static,
    {
        Self {
            status: None,
            headers: None,
            version: None,
            message: Some(context.to_string()),
            trace: Some(error.to_string()),
        }
    }

    pub(crate) fn into_inner_response(self, verbose: bool) -> InnerResponse {
        // Always start with user-created message
        let mut msg: String =  self.message.unwrap_or_else(|| "".to_string());

        if verbose {
            // must be a less awkward way to do this.
            if let Some(trace) = self.trace {
                if msg != "" {
                    msg = msg + "\n" + &trace;
                } else {
                    msg = trace;
                }
            }
        }

        InnerResponse {
            status: self.status.unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            headers: self.headers.unwrap_or_else(HeaderMap::new),
            version: self.version.unwrap_or(Version::HTTP_11),
            body: msg.into(),
        }
    }

    pub fn status(&mut self, status: http::StatusCode) {
        self.status = Some(status);
    }

    pub fn message(&mut self, message: &str) {
        self.message = Some(message.into());
    }

    pub fn bad_request() -> Self {
        Self {
            status: Some(StatusCode::BAD_REQUEST),
            headers: None,
            version: None,
            message: None,
            trace: None,
        }
    }

    pub fn internal_server_error() -> Self {
        Self {
            status: None,
            headers: None,
            version: None,
            message: None,
            trace: None,
        }
    }
}

// Context trait. Will set the `message` field in a glitch
// Design from anyhow

mod private {
    pub trait Sealed {}

    impl<T, E> Sealed for std::result::Result<T, E>
    where
        E: std::error::Error + Send + Sync + 'static
    {}

    impl<T> Sealed for Option<T> {}
}


pub trait Context<T, E>: private::Sealed {
    fn context<C>(self, context: C) -> std::result::Result<T, Glitch>
    where
        C: Display + Send + Sync + 'static;

    fn with_context<C, F>(self, f: F) -> std::result::Result<T, Glitch>
    where
        C: Display + Send + Sync + 'static,
        F: FnOnce() -> C;
}

impl<T, E> Context<T, E> for std::result::Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn context<C>(self, context: C) -> std::result::Result<T, Glitch>
    where
        C: Display + Send + Sync + 'static,
    {
        self.map_err(|error| Glitch::new_with_err_context(error, context))
    }

    fn with_context<C, F>(self, f: F) -> std::result::Result<T, Glitch>
    where
        C: Display + Send + Sync + 'static,
        F: FnOnce() -> C,
    {
        self.map_err(|error| Glitch::new_with_err_context(error, f()))
    }
}

impl<T> Context<T, Infallible> for Option<T> {
    fn context<C>(self, context: C) -> std::result::Result<T, Glitch>
    where
        C: Display + Send + Sync + 'static,
    {
        self.ok_or_else(|| Glitch::new_with_context(context))
    }

    fn with_context<C, F>(self, f: F) -> std::result::Result<T, Glitch>
    where
        C: Display + Send + Sync + 'static,
        F: FnOnce() -> C,
    {
        self.ok_or_else(|| Glitch::new_with_context(f()))
    }
}

#[macro_export]
macro_rules! glitch (
    () => {
        Glitch::internal_server_error();
    };
    ($code:expr) => {
        {
            let mut g= Glitch::new();
            g.status($code);
            g
        }
    };
    ($code:expr, $context:expr) => {
        {
            let mut g= Glitch::new();
            g.status($code);
            g.message($context);
            g
        }
    };
);

#[macro_export]
/// This one panics!
macro_rules! glitch_code (
    () => {
        Glitch::internal_server_error();
    };
    ($code:expr) => {
        {
            let mut g= Glitch::new();
            g.status(StatusCode::from_u16($code).unwrap());
            g
        }
    };
    ($code:expr, $context:expr) => {
        {
            let mut g= Glitch::new();
            g.status(StatusCode::from_u16($code).unwrap());
            g.message($context);
            g
        }
    };
);
