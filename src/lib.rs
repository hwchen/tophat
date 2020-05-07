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
pub use crate::response::{ResponseWriter, ResponseWritten};

/// Accept a new incoming Http/1.1 connection
pub async fn accept<RW, F, Fut>(addr: &str, io: RW, endpoint: F) -> Result<()>
where
    RW: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
    F: Fn(Request, ResponseWriter<RW>) -> Fut,
    Fut: Future<Output = Result<ResponseWritten>>,
{
    loop {
        // decode to Request
        let req_fut = decode(addr, io.clone());

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

        // encode from Response happens when `ResponseWriter::send()` is called inside endpoint
        let resp_wtr = ResponseWriter { writer: io.clone() };
        endpoint(req, resp_wtr).await?;
    }

    Ok(())
}
