// just the server for now

mod body;
mod decode;
mod encode;
mod error;
mod request;
mod response;
mod util;

use futures_core::Future;
use futures_io::{AsyncRead, AsyncWrite};

pub use crate::body::Body;
use crate::decode::decode;
pub use crate::error::{Error, Result};
pub use crate::request::Request;
use crate::response::InnerResponse;
pub use crate::response::{ResponseWriter, ResponseWritten};

/// Accept a new incoming Http/1.1 connection
///
/// Automatically support KeepAlive
// All errors should be bubbled up to this fn to handle, either in logs or in responses.
pub async fn accept<RW, F, Fut>(io: RW, endpoint: F) -> Result<()>
where
    RW: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
    F: Fn(Request, ResponseWriter<RW>) -> Fut,
    Fut: Future<Output = Result<ResponseWritten>>,
{
    loop {
        // decode to Request
        let req_fut = decode(io.clone());

        // Handle req failure modes, timeout, eof
        let req = req_fut.await;
        let req= match req {
            Ok(r) => {
                match r {
                    Some(r) => r,
                    None => break, /* EOF */
                }
            },
            Err(err) => {
                // send a resp for errors from decoding, and continue on to next request
                if let Some(err_resp) = decode::fail_to_response_and_log(err) {
                    let _ = err_resp.send(io.clone()).await;
                }
                continue;
            },
        };

        // Encode from Response happens when `ResponseWriter::send()` is called inside endpoint.
        // Handle errors from:
        // - encoding (std::io::Error): try to send 500
        // - errors from endpoint: send 500
        //
        // Users of tophat should build their own error responses.
        // Perhaps later I can build in a hook for custom error handling, but I should wait for use
        // cases.
        let resp_wtr = ResponseWriter { writer: io.clone() };
        if endpoint(req, resp_wtr).await.is_err() {
            let _ = InnerResponse::internal_server_error()
                .send(io.clone()).await;
        }
    }

    Ok(())
}
