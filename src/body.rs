use futures_lite::{AsyncBufRead, AsyncRead, AsyncReadExt};
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::trailers::{Trailers, TrailersSender};
use crate::util::{empty, Cursor};
use self::error::BodyError;

pin_project_lite::pin_project! {
    /// A streaming body for use with requests and responses.
    ///
    /// includes many convenience methods for converting to and from body
    pub struct Body {
        #[pin]
        pub(crate) reader: Box<dyn AsyncBufRead + Unpin + Send + Sync + 'static>,
        pub(crate) length: Option<usize>,
        trailer_sender: Option<async_channel::Sender<Result<Trailers, BodyError>>>,
        trailer_receiver: async_channel::Receiver<Result<Trailers, BodyError>>,
    }
}

impl Body {
    /// Create an empty Body
    pub fn empty() -> Self {
        let (sender, receiver) = async_channel::bounded(1);

        Self {
            reader: Box::new(empty()),
            length: Some(0),
            trailer_sender: Some(sender),
            trailer_receiver: receiver,
        }
    }

    /// Create a Body from a typ implementing AsyncRead
    ///
    /// if len: None will result in Transfer-Encoding: chunked
    /// if len: Some(n) will result in fixed body
    pub fn from_reader(
        reader: impl AsyncBufRead + Unpin + Send + Sync + 'static,
        len: Option<usize>,
    ) -> Self {
        let (sender, receiver) = async_channel::bounded(1);

        Self {
            reader: Box::new(reader),
            length: len,
            trailer_sender: Some(sender),
            trailer_receiver: receiver,
        }
    }

    /// Create a Body from bytes
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        let (sender, receiver) = async_channel::bounded(1);

        Self {
            length: Some(bytes.len()),
            reader: Box::new(Cursor::new(bytes)),
            trailer_sender: Some(sender),
            trailer_receiver: receiver,
        }
    }

    /// Read a Body into bytes. Consumes Body.
    pub async fn into_bytes(mut self) -> Result<Vec<u8>, BodyError> {
        let mut buf = Vec::with_capacity(1024);
        self.read_to_end(&mut buf)
            .await
            .map_err(BodyError::Conversion)?;
        Ok(buf)
    }

    /// Read a Body into a String. Consumes Body.
    pub async fn into_string(mut self) -> Result<String, BodyError> {
        let mut buf = String::with_capacity(self.length.unwrap_or(0));
        self.read_to_string(&mut buf)
            .await
            .map_err(BodyError::Conversion)?;
        Ok(buf)
    }

    /// sending trailers not yet supported
    pub async fn into_bytes_with_trailer(
        mut self,
    ) -> Result<(Vec<u8>, Option<Result<Trailers, BodyError>>), BodyError> {
        let mut buf = Vec::with_capacity(1024);
        self.read_to_end(&mut buf)
            .await
            .map_err(BodyError::Conversion)?;
        let trailer = self.recv_trailers().await;
        Ok((buf, trailer))
    }

    /// sending trailers not yet supported
    pub async fn into_string_with_trailer(
        mut self,
    ) -> Result<(String, Option<Result<Trailers, BodyError>>), BodyError> {
        let mut buf = String::with_capacity(self.length.unwrap_or(0));
        self.read_to_string(&mut buf)
            .await
            .map_err(BodyError::Conversion)?;
        let trailer = self.recv_trailers().await;
        Ok((buf, trailer))
    }

    /// sending trailers not yet supported
    pub fn send_trailers(&mut self) -> TrailersSender {
        let sender = self
            .trailer_sender
            .take()
            .expect("Trailers sender can only be constructed once");
        TrailersSender::new(sender)
    }

    /// Don't use this directly if you also want to read the body.
    /// In that case, prefer `into_{bytes, string}_with_trailer()
    pub async fn recv_trailers(&self) -> Option<Result<Trailers, BodyError>> {
        self.trailer_receiver.recv().await.ok()
    }

    pub(crate) fn set_inner(
        &mut self,
        rdr: impl AsyncBufRead + Unpin + Send + Sync + 'static,
        len: Option<usize>,
    ) {
        self.reader = Box::new(rdr);
        self.length = len;
    }
}

impl From<String> for Body {
    fn from(s: String) -> Self {
        let (sender, receiver) = async_channel::bounded(1);

        Self {
            length: Some(s.len()),
            reader: Box::new(Cursor::new(s.into_bytes())),
            trailer_sender: Some(sender),
            trailer_receiver: receiver,
        }
    }
}

impl<'a> From<&'a str> for Body {
    fn from(s: &'a str) -> Self {
        let (sender, receiver) = async_channel::bounded(1);

        Self {
            length: Some(s.len()),
            reader: Box::new(Cursor::new(s.to_owned().into_bytes())),
            trailer_sender: Some(sender),
            trailer_receiver: receiver,
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
    fn poll_fill_buf(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<&'_ [u8]>> {
        let this = self.project();
        this.reader.poll_fill_buf(cx)
    }

    fn consume(mut self: Pin<&mut Self>, amt: usize) {
        Pin::new(&mut self.reader).consume(amt)
    }
}

pub mod error {
    use std::fmt;

    /// Error for Body Type
    #[derive(Debug)]
    pub enum BodyError {
        /// Error when converting from a type to Body
        Conversion(std::io::Error),
        /// Error for sending or receiving trailer
        Trailer(std::io::Error),
    }

    impl std::error::Error for BodyError {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            use BodyError::*;
            match self {
                Conversion(err) => Some(err),
                Trailer(err) => Some(err),
            }
        }
    }

    impl fmt::Display for BodyError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            use BodyError::*;
            match self {
                Conversion(err) => write!(f, "Error converting body: {}", err),
                Trailer(err) => write!(f, "Error in body trailer: {}", err),
            }
        }
    }
}
