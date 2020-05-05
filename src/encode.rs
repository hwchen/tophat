use futures_io::AsyncRead;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::body::Body;

pub struct Encoder<'a> {
    body: &'a Body,
    #[allow(dead_code)]
    bytes_read: usize, // for tracking total bytes read
    times_read: usize,
}

impl<'a> Encoder<'a> {
    pub fn encode(body: &'a Body) -> Self {
        Self {
            body,
            bytes_read: 0,
            times_read: 0,
        }
    }
}

impl<'a> AsyncRead for Encoder<'a> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        if self.times_read == 0 {
            // just send the whole body at once
            let bytes = self.body.as_bytes().unwrap().unwrap();
            let len = bytes.len();
            std::io::Read::read(&mut std::io::Cursor::new(bytes), buf).unwrap();
            dbg!(&buf[..15]);
            self.times_read += 1;
            Poll::Ready(Ok(len))
        } else {
            Poll::Ready(Ok(0))
        }
    }
}
