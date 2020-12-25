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

use http::{header::HeaderMap, status::StatusCode, version::Version};
use std::convert::Infallible;
use std::fmt::Display;

use crate::server::InnerResponse;

/// Convenience type for `Result<T, Glitch>`
pub type Result<T> = std::result::Result<T, Glitch>;

// similar to inner_response, but with string-only body
/// Glitch is designed to be the error response for tophat. Users can either create them manually,
/// or use `GlitchExt` to easily convert from `std::error::Error`.
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
    /// Create a Glitch
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

    pub(crate) fn new_with_status_context<C>(status: StatusCode, context: C) -> Self
    where
        C: Display + Send + Sync + 'static,
    {
        Self {
            status: Some(status),
            headers: None,
            version: None,
            message: Some(context.to_string()),
            trace: None,
        }
    }

    pub(crate) fn new_with_status_err_context<E, C>(
        status: StatusCode,
        error: E,
        context: C,
    ) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
        C: Display + Send + Sync + 'static,
    {
        Self {
            status: Some(status),
            headers: None,
            version: None,
            message: Some(context.to_string()),
            trace: Some(error.to_string()),
        }
    }

    pub(crate) fn into_inner_response(self, verbose: bool) -> InnerResponse {
        // Always start with user-created message
        let mut msg: String = self.message.unwrap_or_else(|| "".to_string());

        if verbose {
            // must be a less awkward way to do this.
            if let Some(trace) = self.trace {
                #[allow(clippy::comparison_to_empty)]
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

    /// Set status of a Glitch
    pub fn status(&mut self, status: http::StatusCode) {
        self.status = Some(status);
    }

    /// Add a message to a Glitch
    pub fn message(&mut self, message: &str) {
        self.message = Some(message.into());
    }

    /// Convenience method for sending a 400
    pub fn bad_request() -> Self {
        Self {
            status: Some(StatusCode::BAD_REQUEST),
            headers: None,
            version: None,
            message: None,
            trace: None,
        }
    }

    /// Convenience method for sending a 500
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

    impl<T, E> Sealed for std::result::Result<T, E> where E: std::error::Error + Send + Sync + 'static {}

    impl<T> Sealed for Option<T> {}
}

/// GlitchExt makes it easy to chain onto a Result or Option, and convert into a Glitch.
pub trait GlitchExt<T, E>: private::Sealed {
    /// chain with `.glitch(<StatusCode>)?`, sets a Glitch with empty body.
    fn glitch(self, status: StatusCode) -> std::result::Result<T, Glitch>;

    /// chain with `.glitch_ctx(<StatusCode>, "your_msg")?`, sets a Glitch with message in body.
    fn glitch_ctx<C>(self, status: StatusCode, ctx: C) -> std::result::Result<T, Glitch>
    where
        C: Display + Send + Sync + 'static;

    /// chain with `.glitch_ctx(<StatusCode>, || x.to_string())?`, sets a Glitch with message in body.
    ///
    /// Use when your context is set using a function, instead of just a value.
    fn glitch_with_ctx<C, F>(self, status: StatusCode, f: F) -> std::result::Result<T, Glitch>
    where
        C: Display + Send + Sync + 'static,
        F: FnOnce() -> C;
}

impl<T, E> GlitchExt<T, E> for std::result::Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn glitch(self, status: StatusCode) -> std::result::Result<T, Glitch> {
        self.map_err(|_| {
            let mut g = Glitch::new();
            g.status(status);
            g
        })
    }

    fn glitch_ctx<C>(self, status: StatusCode, context: C) -> std::result::Result<T, Glitch>
    where
        C: Display + Send + Sync + 'static,
    {
        self.map_err(|error| Glitch::new_with_status_err_context(status, error, context))
    }

    fn glitch_with_ctx<C, F>(self, status: StatusCode, f: F) -> std::result::Result<T, Glitch>
    where
        C: Display + Send + Sync + 'static,
        F: FnOnce() -> C,
    {
        self.map_err(|error| Glitch::new_with_status_err_context(status, error, f()))
    }
}

impl<T> GlitchExt<T, Infallible> for Option<T> {
    fn glitch(self, status: StatusCode) -> std::result::Result<T, Glitch> {
        self.ok_or_else(|| {
            let mut g = Glitch::new();
            g.status(status);
            g
        })
    }

    fn glitch_ctx<C>(self, status: StatusCode, context: C) -> std::result::Result<T, Glitch>
    where
        C: Display + Send + Sync + 'static,
    {
        self.ok_or_else(|| Glitch::new_with_status_context(status, context))
    }

    fn glitch_with_ctx<C, F>(self, status: StatusCode, f: F) -> std::result::Result<T, Glitch>
    where
        C: Display + Send + Sync + 'static,
        F: FnOnce() -> C,
    {
        self.ok_or_else(|| Glitch::new_with_status_context(status, f()))
    }
}

/// Convenience macro for creating a Glitch.
///
/// `glitch!()`: 500
/// `glitch!(StatusCode::BadRequest)`: 400
/// `glitch!(StatusCode::BadRequest, "custom error")`: 400 with message in body
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
///
/// Convenience macro for creating a Glitch.
///
/// `glitch_code!()`: 500
/// `glitch_code!(400)`: 400
/// `glitch_code!(400, "custom error")`: 400 with message in body
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
