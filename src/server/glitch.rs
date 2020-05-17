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
//! - Just use `?` on any error, and it will be turned into a 500 response.
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

//use crate::server::InnerResponse;


pub type Result<T> = std::result::Result<T, Glitch>;

// similar to inner_response, but with string-only body
#[derive(Debug)]
pub struct Glitch {
    pub(crate) status: Option<StatusCode>,
    pub(crate) headers: Option<HeaderMap>,
    pub(crate) version: Option<Version>,
    pub(crate) message: Option<String>,
}

// convert From Glitch to InnerResponse
