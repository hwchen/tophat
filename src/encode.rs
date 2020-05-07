// TODO support more than fixed length body
//
// Note: I fixed the encoding ranges on the buffer, and used bytes_read correctly.
// But the final buffer ended up the same? I guess that sending the wrong number of bytes read
// must have mucked up what the stream was reading back out.

use futures_io::AsyncRead;
use httpdate::fmt_http_date;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::response::InnerResponse;

pub(crate) struct Encoder {
    resp: InnerResponse,
    state: EncoderState,

    // Tracks bytes read across one Encoder poll_read, which may span
    // several calls of encoding methods
    bytes_read: usize,

    head_buf: Vec<u8>,
    head_bytes_read: usize,

    content_length: Option<usize>,
    body_bytes_read: usize,
}

impl Encoder {
    pub(crate) fn encode(resp: InnerResponse) -> Self {
        let content_length = resp.body.length;

        Self {
            resp,
            state: EncoderState::Start,
            bytes_read: 0,
            head_buf: Vec::new(),
            head_bytes_read: 0,
            content_length,
            body_bytes_read: 0,
        }
    }

    /// At start, prep headers for writing
    fn start(&mut self, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<std::io::Result<usize>> {
        let version = self.resp.version;
        let status = self.resp.status;
        // TODO deal with date later
        let date = fmt_http_date(std::time::SystemTime::now());

        std::io::Write::write_fmt(&mut self.head_buf, format_args!("{:?} {}\r\n", version, status))?;
        if let Some(len) = self.content_length {
            std::io::Write::write_fmt(&mut self.head_buf, format_args!("content-length: {}\r\n", len))?;
        } else {
            std::io::Write::write_fmt(&mut self.head_buf, format_args!("transfer-encoding: chunked\r\n"))?;
        }
        std::io::Write::write_fmt(&mut self.head_buf, format_args!("date: {}\r\n", date)).unwrap();
        for (header, value) in &self.resp.headers {
            // TODO check this: shouldn't head be &HeaderName, not Option<HeaderName>?
            std::io::Write::write_fmt(&mut self.head_buf, format_args!("{}: {}\r\n", header, value.to_str().unwrap()))?;
        }
        std::io::Write::write_fmt(&mut self.head_buf, format_args!("\r\n"))?;

        // Now everything's prepped, on to sending the header
        self.state = EncoderState::Head;
        self.encode_head(cx, buf)
    }

    fn encode_head(&mut self, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<std::io::Result<usize>> {
        // Each read is not guaranteed to read the entire head_buf. So we keep track of our place
        // if the read is partial, so that it can be continued on the next poll.

        // Copy to to buf the shorter of (remaining head_buf) or buf
        let len = std::cmp::min(
            self.head_buf.len() - self.head_bytes_read,
            buf.len()
        );
        let range = self.head_bytes_read..self.head_bytes_read + len;
        buf[0..len].copy_from_slice(&self.head_buf[range]);
        self.bytes_read += len;
        self.head_bytes_read += len;

        // if entire head_buf is read, continue to body encoding, else keep state and return
        // Poll::Ready for this iteration
        if self.head_bytes_read == self.head_buf.len() {
            match self.content_length {
                Some(_) => {
                    self.state = EncoderState::FixedBody;
                    self.encode_fixed_body(cx, buf)
                }
                None => {
                    // TODO for now just end
                    Poll::Ready(Ok(self.bytes_read))
                }
            }
        } else {
            Poll::Ready(Ok(self.bytes_read))
        }
    }

    fn encode_fixed_body(&mut self, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<std::io::Result<usize>> {
        // Remember that from here, the buf has not been cleared yet, so consider the head as the
        // first part of the buf.

        // first check that there's more room in buffer
        if self.bytes_read == buf.len() {
            return Poll::Ready(Ok(self.bytes_read));
        }

        let content_length = self.content_length.unwrap();

        // Copy to to buf the shorter of (remaining body + any previous reads) or buf
        let upper_limit = std::cmp::min(
            self.bytes_read + content_length - self.body_bytes_read,
            buf.len()
        );
        let range = self.bytes_read..upper_limit;
        let inner_read = Pin::new(&mut self.resp.body).poll_read(cx, &mut buf[range]);
        match inner_read {
            Poll::Ready(Ok(n)) => {
                self.bytes_read += n;
                self.body_bytes_read += n;
            },
            Poll::Ready(Err(err)) => {
                return Poll::Ready(Err(err));
            },
            Poll::Pending => {
                 match self.bytes_read {
                      0 => return Poll::Pending,
                      n => return Poll::Ready(Ok(n)),
                 }
            },
        }

        // if entire resp is read, finish. Else return Poll::Ready for another iteration
        if content_length == self.body_bytes_read {
            self.state = EncoderState::Done;
            Poll::Ready(Ok(self.bytes_read))
        } else {
            self.encode_fixed_body(cx, buf)
        }
    }
}

impl AsyncRead for Encoder {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        // bytes_read is per call to poll_read for Encoder
        self.bytes_read = 0;

        use EncoderState::*;
        match self.state {
            Start => self.start(cx, buf),
            Head => self.encode_head(cx, buf),
            FixedBody => self.encode_fixed_body(cx, buf),
            Done => Poll::Ready(Ok(0)),
        }
    }
}

#[derive(Debug)]
enum EncoderState {
    Start,
    Head,
    FixedBody,
    Done,
}
