use futures_io::AsyncRead;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::response::InnerResponse;

pub struct Encoder {
    resp: InnerResponse,
    #[allow(dead_code)]
    bytes_read: usize, // for tracking total bytes read
}

impl Encoder {
    pub(crate) fn encode(resp: InnerResponse) -> Self {
        Self {
            resp,
            bytes_read: 0,
        }
    }
}

impl AsyncRead for Encoder {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.resp.body).poll_read(cx, buf)
    }
}
