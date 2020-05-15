use futures_io::AsyncWrite;
use http::{
    header::{HeaderMap, HeaderName, HeaderValue},
    status::StatusCode,
    version::Version,
};
use thiserror::Error as ThisError;

use crate::body::Body;
use crate::error::Error;
use crate::response::Response;

use super::encode::Encoder;

pin_project_lite::pin_project! {
    pub(crate) struct InnerResponse {
        pub(crate) status: StatusCode,
        pub(crate) headers: HeaderMap,
        //url: Url, // TODO what is this for?
        pub(crate) version: Version,
        //pub(crate) extensions: Extensions, // TODO do I need this?
        #[pin]
        pub(crate)body: Body,
    }
}

impl InnerResponse {
    /// used for bad request in decoding. 400
    pub(crate) fn bad_request() -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            headers: HeaderMap::new(),
            version: Version::default(),
            body: Body::empty(),
        }
    }

    /// used for bad request in decoding. 500
    pub(crate) fn internal_server_error() -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            headers: HeaderMap::new(),
            version: Version::default(),
            body: Body::empty(),
        }
    }

    /// used for version not supported in decoding. 505
    pub(crate) fn version_not_supported() -> Self {
        Self {
            status: StatusCode::HTTP_VERSION_NOT_SUPPORTED,
            headers: HeaderMap::new(),
            version: Version::default(),
            body: Body::empty(),
        }
    }

    pub(crate) async fn send<W>(self, writer: W) -> Result<ResponseWritten, ResponseFail>
        where W: AsyncWrite + Clone + Send + Sync + Unpin + 'static,
    {
        let mut encoder = Encoder::encode(self);
        let mut writer = writer;
        futures_util::io::copy(&mut encoder, &mut writer).await.map_err(ResponseFail::Connection)?;
        Ok(ResponseWritten)
    }
}

pub struct ResponseWriter<W>
where
    W: AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    // TODO make not public
    pub response: Response,
    pub writer: W,
}

impl<W> ResponseWriter<W>
where
    W: AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    /// send response, and TODO return number of bytes written (I guess this would be a struct for more
    /// complicated sends, like with compression)
    pub async fn send(self) -> Result<ResponseWritten, Error> {
        let (parts, body) = self.response.into_parts();

        let inner_resp = InnerResponse {
            status: parts.status,
            headers: parts.headers,
            version: parts.version,
            body,
        };

        Ok(inner_resp.send(self.writer).await?)
    }

    pub async fn send_code(self, code: u16) -> Result<ResponseWritten, Error> {
        let mut this = self;
        this.set_code(code);

        this.send().await
    }

    pub fn set_status(&mut self, status: http::StatusCode) -> &mut Self {
        *self.response.status_mut() = status;
        self
    }

    /// Internally panics if code is incorrect
    pub fn set_code(&mut self, code: u16) -> &mut Self {
        *self.response.status_mut() = http::StatusCode::from_u16(code).unwrap();
        self
    }

    pub fn set_body(&mut self, body: Body) -> &mut Self {
        *self.response.body_mut() = body;
        self
    }

    pub fn append_header(&mut self, header_name: HeaderName, header_value: HeaderValue) -> &mut Self {
        self.response.headers_mut().append(header_name, header_value);
        self
    }

    pub fn insert_header(&mut self, header_name: HeaderName, header_value: HeaderValue) -> &mut Self {
        self.response.headers_mut().insert(header_name, header_value);
        self
    }

    // mutable access to the full response
    pub fn response_mut(&mut self) -> &mut Response {
        &mut self.response
    }

    pub fn set_text(&mut self, text: String) -> &mut Self {
        *self.response.body_mut() = text.into();
        self.response.headers_mut().insert(http::header::CONTENT_TYPE, "text/plain".parse().unwrap());
        self
    }
}

// TODO have a ReponseResult, which may contain bytes read etc. And then have it transform into
// ResponseWritten, to minimize boilerplate
pub struct ResponseWritten;

#[derive(ThisError, Debug)]
pub(crate) enum ResponseFail {
    #[error("Failure sending response: {0}")]
    Connection(std::io::Error),
}

impl From<ResponseFail> for Error {
    fn from(respf: ResponseFail) -> Error {
        match respf {
            ResponseFail::Connection(io_err) => Error::ResponseSend(io_err),
        }
    }
}

