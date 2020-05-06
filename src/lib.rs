// just the server for now

mod body;
mod date;
mod decode;
mod encode;
mod request;
mod response;
mod util;

use futures_core::Future;
use futures_io::{AsyncRead, AsyncWrite};

pub use crate::body::Body;
use crate::decode::decode;
pub use crate::request::Request;
pub use crate::response::{ResponseWriter, ResponseWritten};

/// Accept a new incoming Http/1.1 connection
pub async fn accept<RW, F, Fut>(addr: &str, io: RW, endpoint: F) -> http::Result<()>
where
    RW: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
    F: Fn(Request, ResponseWriter<RW>) -> Fut,
    Fut: Future<Output = http::Result<ResponseWritten>>,
{
    loop {
        // decode to Request
        let req_fut = decode(addr, io.clone());

        // Handle eof
        let req = match req_fut.await? {
            Some(r) => r,
            None => break, /* EOF */
        };

        // encode from Response happens when `ResponseWriter::send()` is called inside endpoint
        let resp_wtr = ResponseWriter { writer: io.clone() };
        endpoint(req, resp_wtr).await?;
    }

    Ok(())
}
