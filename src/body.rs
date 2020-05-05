use futures_io::{AsyncRead, AsyncBufRead};
use futures_util::io::AsyncReadExt;
use mime::{self, Mime};
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::util::{empty, Cursor};

pin_project_lite::pin_project! {
    pub struct Body {
        #[pin]
        reader: Box<dyn AsyncBufRead + Unpin + Send + Sync + 'static>,
        mime: Mime,
        length: Option<usize>,
    }
}

impl Body {
    pub fn empty() -> Self {
        Self {
            reader: Box::new(empty()),
            mime: mime::APPLICATION_OCTET_STREAM,
            length: Some(0),
        }
    }

    pub fn from_reader(
        reader: impl AsyncBufRead + Unpin + Send + Sync + 'static,
        len: Option<usize>,
    ) -> Self {
        Self {
            reader: Box::new(reader),
            mime: mime::APPLICATION_OCTET_STREAM,
            length: len,
        }
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self {
            length: Some(bytes.len()),
            reader: Box::new(Cursor::new(bytes)),
            mime: mime::APPLICATION_OCTET_STREAM,
        }
    }

    // TODO make errors
    pub async fn into_bytes(mut self) -> Result<Vec<u8>, std::io::Error> {
        let mut buf = Vec::with_capacity(1024);
        self.read_to_end(&mut buf).await?;
        Ok(buf)
    }

    pub async fn into_string(mut self) -> Result<String, std::io::Error> {
        let mut buf = String::with_capacity(self.length.unwrap_or(0));
        self.read_to_string(&mut buf).await?;
        Ok(buf)
    }
}

impl From<String> for Body {
    fn from(s: String) -> Self {
        Self {
            length: Some(s.len()),
            reader: Box::new(Cursor::new(s.into_bytes())),
            mime: mime::TEXT_PLAIN,
        }
    }
}

impl AsyncRead for Body {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.reader).poll_read(cx, buf)
    }
}

impl AsyncBufRead for Body {
    fn poll_fill_buf(self: Pin<&mut Self>, cx: &mut Context<'_>,) -> Poll<io::Result<&'_ [u8]>> {
        let this = self.project();
        this.reader.poll_fill_buf(cx)
    }

    fn consume(mut self: Pin<&mut Self>, amt: usize) {
        Pin::new(&mut self.reader).consume(amt)
    }
}
