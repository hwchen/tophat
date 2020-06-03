use futures_util::io::{AsyncRead, AsyncWriteExt};
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use http::{header::HOST, Method, Request};

use crate::Body;
use super::{error, ClientError};

/// An HTTP encoder.
#[doc(hidden)]
pub struct Encoder {
    /// Keep track how far we've indexed into the headers + body.
    cursor: usize,
    /// HTTP headers to be sent.
    headers: Vec<u8>,
    /// Check whether we're done sending headers.
    headers_done: bool,
    /// HTTP body to be sent.
    body: Body,
    /// Check whether we're done with the body.
    body_done: bool,
    /// Keep track of how many bytes have been read from the body stream.
    body_bytes_read: usize,
}

impl Encoder {
    /// Encode an HTTP request on the client.
    pub async fn encode(req: Request<Body>) -> Result<Self, ClientError> {
        let mut buf: Vec<u8> = vec![];

        // clients are not supposed to send uri frags when retrieving a document
        // removed code for that here, skip to query.
        let mut url = req.uri().path().to_owned();
        if let Some(query) = req.uri().query() {
            url.push('?');
            url.push_str(query);
        }

        // A client sending a CONNECT request MUST consists of only the host
        // name and port number of the tunnel destination, separated by a colon.
        // See: https://tools.ietf.org/html/rfc7231#section-4.3.6
        if req.method() == Method::CONNECT {
            let host = req.uri().host();
            let host = host.ok_or_else(|| error::encode("Missing hostname".to_owned()))?;
            let port = req.uri().port(); // or known default?
            let port = port.ok_or_else(|| error::encode("Missing port".to_owned()))?;
            url = format!("{}:{}", host, port);
        }

        let val = format!("{} {} HTTP/1.1\r\n", req.method(), url);
        log::trace!("> {}", &val);
        buf.write_all(val.as_bytes()).await
            .map_err(error::encode_io)?;

        if req.headers().get(HOST).is_none() {
            // Insert Host header
            // Insert host
            let host = req.uri().host();
            let host = host.ok_or_else(|| error::encode("Missing hostname".to_owned()))?;
            let val = if let Some(port) = req.uri().port() {
                format!("host: {}:{}\r\n", host, port)
            } else {
                format!("host: {}\r\n", host)
            };

            log::trace!("> {}", &val);
            buf.write_all(val.as_bytes()).await
                .map_err(error::encode_io)?;
        }

        // Insert Proxy-Connection header when method is CONNECT
        if req.method() == Method::CONNECT {
            let val = "proxy-connection: keep-alive\r\n".to_owned();
            log::trace!("> {}", &val);
            buf.write_all(val.as_bytes()).await
                .map_err(error::encode_io)?;
        }

        // If the body isn't streaming, we can set the content-length ahead of time. Else we need to
        // send all items in chunks.
        if let Some(len) = req.body().length {
            let val = format!("content-length: {}\r\n", len);
            log::trace!("> {}", &val);
            buf.write_all(val.as_bytes()).await
                .map_err(error::encode_io)?;
        } else {
            // write!(&mut buf, "Transfer-Encoding: chunked\r\n")?;
            panic!("chunked encoding is not implemented yet");
            // See: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Transfer-Encoding
            //      https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Trailer
        }

        for (header, value) in req.headers().iter() {
            buf.write_all(header.as_str().as_bytes()).await
                .map_err(error::encode_io)?;
            buf.write_all(b": ").await
                .map_err(error::encode_io)?;
            buf.write_all(value.as_bytes()).await
                .map_err(error::encode_io)?;
            buf.write_all(b"\r\n").await
                .map_err(error::encode_io)?;
        }

        buf.write_all(b"\r\n").await
            .map_err(error::encode_io)?;

        Ok(Self {
            body: req.into_body(),
            headers: buf,
            cursor: 0,
            headers_done: false,
            body_done: false,
            body_bytes_read: 0,
        })
    }
}

impl AsyncRead for Encoder {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        // Send the headers. As long as the headers aren't fully sent yet we
        // keep sending more of the headers.
        let mut bytes_read = 0;
        if !self.headers_done {
            let len = std::cmp::min(self.headers.len() - self.cursor, buf.len());
            let range = self.cursor..self.cursor + len;
            buf[0..len].copy_from_slice(&self.headers[range]);
            self.cursor += len;
            if self.cursor == self.headers.len() {
                self.headers_done = true;
            }
            bytes_read += len;
        }

        if !self.body_done {
            let inner_poll_result =
                Pin::new(&mut self.body).poll_read(cx, &mut buf[bytes_read..]);
            let n = match inner_poll_result {
                Poll::Ready(Ok(n)) => n,
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => {
                    if bytes_read == 0 {
                        return Poll::Pending;
                    } else {
                        return Poll::Ready(Ok(bytes_read as usize));
                    }
                }
            };
            bytes_read += n;
            self.body_bytes_read += n;
            if bytes_read == 0 {
                self.body_done = true;
            }
        }

        Poll::Ready(Ok(bytes_read as usize))
    }
}
