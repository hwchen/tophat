use futures_io::{AsyncRead, AsyncBufRead};
use futures_util::io::AsyncReadExt;
use mime::{self, Mime};
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

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
    pub async fn into_bytes(mut self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut buf = Vec::with_capacity(1024);
        self.read_to_end(&mut buf).await?;
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

struct Cursor<T> {
    inner: std::io::Cursor<T>,
}

impl<T> Cursor<T> {
    fn new(t: T) -> Self {
        Self {
            inner: std::io::Cursor::new(t),
        }
    }
}

impl<T> AsyncRead for Cursor<T>
where
    T: AsRef<[u8]> + Unpin,
{
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        Poll::Ready(std::io::Read::read(&mut self.inner, buf))
    }

    fn poll_read_vectored(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        bufs: &mut [std::io::IoSliceMut<'_>],
    ) -> Poll<std::io::Result<usize>> {
        Poll::Ready(std::io::Read::read_vectored(&mut self.inner, bufs))
    }
}

impl<T> AsyncBufRead for Cursor<T>
where
    T: AsRef<[u8]> + Unpin,
{
    fn poll_fill_buf(self: Pin<&mut Self>, _cx: &mut Context<'_>,) -> Poll<io::Result<&'_ [u8]>> {
        Poll::Ready(std::io::BufRead::fill_buf(&mut self.get_mut().inner))
    }

    fn consume(mut self: Pin<&mut Self>, amt: usize) {
        std::io::BufRead::consume(&mut self.inner, amt)
    }
}

struct Empty;

fn empty() -> Empty {
    Empty
}

impl AsyncRead for Empty {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        Poll::Ready(Ok(0))
    }

    fn poll_read_vectored(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _bufs: &mut [std::io::IoSliceMut<'_>],
    ) -> Poll<std::io::Result<usize>> {
        Poll::Ready(Ok(0))
    }
}

impl AsyncBufRead for Empty {
    fn poll_fill_buf(self: Pin<&mut Self>, _cx: &mut Context<'_>,) -> Poll<io::Result<&'_ [u8]>> {
        Poll::Ready(Ok(&[]))
    }

    fn consume(self: Pin<&mut Self>, _amt: usize) {}
}
