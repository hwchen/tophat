#![deny(unsafe_code)]

//! # tophat server

#[cfg(feature = "cors")]
pub mod cors;
mod decode;
mod encode;
pub mod glitch;
mod response_writer;
#[cfg(feature = "router")]
pub mod router;
#[cfg(feature = "identity")]
pub mod identity;

use futures_core::Future;
use futures_io::{AsyncRead, AsyncWrite};
use std::time::Duration;

use crate::body::Body;
use crate::error::Error;
use crate::request::Request;
use crate::response::Response;
use crate::server::decode::DecodeFail;
use crate::timeout::{timeout, TimeoutError};

use self::decode::decode;
pub use self::glitch::{Glitch, Result};
use self::response_writer::InnerResponse;
pub use self::response_writer::{ResponseWriter, ResponseWritten};

/// Accept a new incoming Http/1.1 connection
///
/// Automatically supports KeepAlive
pub async fn accept<RW, F, Fut>(io: RW, endpoint: F) -> std::result::Result<(), Error>
where
    RW: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
    F: Fn(Request, ResponseWriter<RW>) -> Fut,
    Fut: Future<Output = Result<ResponseWritten>>,
{
    accept_with_opts(io, endpoint, ServerOpts::default()).await
}

/// Accept a new incoming Http/1.1 connection
///
/// Automatically supports KeepAlive
pub async fn accept_with_opts<RW, F, Fut>(io: RW, endpoint: F, opts: ServerOpts) -> std::result::Result<(), Error>
where
    RW: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
    F: Fn(Request, ResponseWriter<RW>) -> Fut,
    Fut: Future<Output = Result<ResponseWritten>>,
{
    // All errors should be bubbled up to this fn to handle, either in logs or in responses.

    loop {
        // decode to Request
        let req_fut = decode(io.clone());

        // Handle req failure modes, timeout, eof
        let req = if let Some(timeout_duration) = opts.timeout {
            // this arm is for with timeout
            match timeout(timeout_duration, req_fut).await {
                Ok(Ok(Some(r))) => r,
                Ok(Ok(None)) | Err(TimeoutError { .. }) => {
                    log::debug!("Timeout Error");
                    break; // EOF or timeout
                },
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
                },
            }
        };

        // Encode from Response happens when `ResponseWriter::send()` is called inside endpoint.
        // Handle errors from:
        // - encoding (std::io::Error): try to send 500
        // - errors from endpoint: send 500
        //
        // Users of tophat should build their own error responses.
        // Perhaps later I can build in a hook for custom error handling, but I should wait for use
        // cases.
        let resp_wtr = ResponseWriter { writer: io.clone(), response: Response::new(Body::empty()) };
        // TODO will spawning task here approximate multiplexing? Ah, but then I need integration
        // with executor.
        if endpoint(req, resp_wtr).await.is_err() {
            let _ = InnerResponse::internal_server_error()
                .send(io.clone()).await;
        }
    }

    Ok(())
}

pub struct ServerOpts {
    pub timeout: Option<Duration>,
}

impl Default for ServerOpts {
    fn default() -> Self {
        Self {
            timeout: Some(Duration::from_secs(60)),
        }
    }
}

// handles both writing response, and currently picking out which errors will return err and close
// connection. TODO could use more infrastructure in decode to name which decode fails should become
// lib-level errors. But it was a bit too boilerplatey to implement in the decode module.
async fn handle_decode_fail<RW>(fail: DecodeFail, io: RW) -> std::result::Result<(), Error>
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
