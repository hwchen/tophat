//! Simple client for HTTP/1.1

mod decode;
mod encode;
mod error;

use futures_util::io::{self, AsyncRead, AsyncWrite};

use crate::{Request, Response};
use decode::decode;
use encode::Encoder;
use error::ClientError;

/// Opens an HTTP/1.1 connection to a remote host.
pub async fn connect<RW>(mut stream: RW, req: Request) -> Result<Response, ClientError>
where
    RW: AsyncRead + AsyncWrite + Send + Sync + Unpin + 'static,
{
    let mut req = Encoder::encode(req).await?;

    io::copy(&mut req, &mut stream).await
        .map_err(error::io)?;

    let res = decode(stream).await?;

    Ok(res)
}
