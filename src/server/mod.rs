#![deny(unsafe_code)]

//! # tophat server

#[cfg(feature = "cors")]
pub mod cors;
mod decode;
mod encode;
pub mod glitch;
#[cfg(feature = "identity")]
pub mod identity;
mod response_writer;
#[cfg(feature = "router")]
pub mod router;
pub mod error;

use futures_core::Future;
use futures_io::{AsyncRead, AsyncWrite};
use std::time::Duration;

use crate::body::Body;
use crate::request::Request;
use crate::response::Response;
use crate::server::decode::DecodeFail;
use crate::timeout::{timeout, TimeoutError};

use self::decode::decode;
pub use self::error::ServerError;
pub use self::glitch::{Glitch, Result};
use self::response_writer::InnerResponse;
pub use self::response_writer::{ResponseWriter, ResponseWritten};

/// Accept a new incoming Http/1.1 connection
///
/// Automatically supports KeepAlive
pub async fn accept<RW, F, Fut>(io: RW, endpoint: F) -> std::result::Result<(), ServerError>
where
    RW: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
    F: Fn(Request, ResponseWriter<RW>) -> Fut,
    Fut: Future<Output = Result<ResponseWritten>>,
{
    accept_with_opts(io, ServerOpts::default(), endpoint).await
}

/// Accept a new incoming Http/1.1 connection
///
/// Automatically supports KeepAlive
pub async fn accept_with_opts<RW, F, Fut>(
    io: RW,
    opts: ServerOpts,
    endpoint: F,
) -> std::result::Result<(), ServerError>
where
    RW: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
    F: Fn(Request, ResponseWriter<RW>) -> Fut,
    Fut: Future<Output = Result<ResponseWritten>>,
{
    // All errors should be bubbled up to this fn to handle, either in logs or in responses.

    loop {
        // decode to Request
        // returns Ok(None) if no request to decode. So no need to exit on a ConnectionLost error.
        let req_fut = decode(io.clone());

        // Handle req failure modes, timeout, eof
        let req = if let Some(timeout_duration) = opts.timeout {
            // this arm is for with timeout
            match timeout(timeout_duration, req_fut).await {
                Ok(Ok(Some(r))) => r,
                Ok(Ok(None)) | Err(TimeoutError { .. }) => {
                    //debug!("Timeout Error");
                    break; // EOF or timeout
                }
                Ok(Err(err)) => {
                    handle_decode_fail(err, io.clone()).await?;
                    // and continue on to next request
                    continue;
                }
            }
        } else {
            // This arm is for no timeout
            match req_fut.await {
                Ok(Some(r)) => r,
                Ok(None) => break, // EOF
                Err(err) => {
                    handle_decode_fail(err, io.clone()).await?;
                    // and continue on to next request
                    continue;
                }
            }
        };

        let resp_wtr = ResponseWriter {
            writer: io.clone(),
            response: Response::new(Body::empty()),
        };
        if let Err(glitch) = endpoint(req, resp_wtr).await {
            let _ = glitch
                .into_inner_response(opts.verbose_glitch)
                .send(io.clone())
                .await;
        }
    }

    Ok(())
}

/// Options for the tophat server.
#[derive(Clone)]
pub struct ServerOpts {
    /// Connection timeout (in seconds)
    pub timeout: Option<Duration>,
    /// Option to send error (from convertin error to Glitch) traces in an error response (Glitch)
    pub verbose_glitch: bool,
}

impl Default for ServerOpts {
    fn default() -> Self {
        Self {
            timeout: Some(Duration::from_secs(60)),
            verbose_glitch: false,
        }
    }
}

// handles both writing error response and bubbling up major system errors as necessary.
async fn handle_decode_fail<RW>(fail: DecodeFail, io: RW) -> std::result::Result<(), ServerError>
where
    RW: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    // send a resp for errors from decoding
    if let Some(err_resp) = decode::fail_to_response_and_log(&fail) {
        let _ = err_resp.send(io.clone()).await;
    }
    // Early return if there's a major error.
    if let Some(crate_err) = decode::fail_to_crate_err(fail) {
        return Err(crate_err);
    }

    Ok(())
}
