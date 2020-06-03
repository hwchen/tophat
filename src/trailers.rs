//! # Trailers
//!
//! Trailers are headers sent at the end of a chunked message.
//!
//! Currently tophat can only receive, not send them.

use async_channel::Sender;
use http::HeaderMap;
use std::ops::{Deref, DerefMut};

use crate::Error;

/// A collection of trailing HTTP headers.
#[derive(Debug)]
pub struct Trailers {
    /// The headers in a trailer
    pub headers: HeaderMap,
}

impl Trailers {
    /// Create a new instance of `Trailers`.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for Trailers {
    fn default() -> Self {
        Self {
            headers: HeaderMap::new(),
        }
    }
}

impl Clone for Trailers {
    fn clone(&self) -> Self {
        Self {
            headers: self.headers.clone(),
        }
    }
}

impl Deref for Trailers {
    type Target = HeaderMap;

    fn deref(&self) -> &Self::Target {
        &self.headers
    }
}

impl DerefMut for Trailers {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.headers
    }
}

/// The sending half of a channel to send trailers.
///
/// Unlike `async_std::sync::channel` the `send` method on this type can only be
/// called once, and cannot be cloned. That's because only a single instance of
/// `Trailers` should be created.
#[derive(Debug)]
pub struct TrailersSender {
    sender: Sender<Result<Trailers, Error>>,
}

impl TrailersSender {
    /// Create a new instance of `TrailersSender`.
    #[doc(hidden)]
    pub(crate) fn new(sender: Sender<Result<Trailers, Error>>) -> Self {
        Self { sender }
    }

    /// Send a `Trailer`.
    ///
    /// The channel will be consumed after having sent trailers.
    pub(crate) async fn send(self, trailers: Result<Trailers, Error>) {
        // TODO should this return an error?
        let _ = self.sender.send(trailers).await;
    }
}
