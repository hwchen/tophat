use futures_lite::{io, AsyncWrite};
use futures_util::TryStreamExt;
use http::{
    header::{HeaderMap, HeaderValue, IntoHeaderName},
    status::StatusCode,
    version::Version,
};
use tracing::error;

use crate::body::Body;
use crate::response::Response;

use super::encode::Encoder;
use super::glitch::Glitch;

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

    /// used for version not supported in decoding. 505
    pub(crate) fn version_not_supported() -> Self {
        Self {
            status: StatusCode::HTTP_VERSION_NOT_SUPPORTED,
            headers: HeaderMap::new(),
            version: Version::default(),
            body: Body::empty(),
        }
    }

    /// used for unimplemented transfer-encoding in decoding. 501
    pub(crate) fn not_implemented() -> Self {
        Self {
            status: StatusCode::NOT_IMPLEMENTED,
            headers: HeaderMap::new(),
            version: Version::default(),
            body: Body::empty(),
        }
    }

    pub(crate) async fn send<W>(self, writer: W) -> Result<ResponseWritten, std::io::Error>
    where
        W: AsyncWrite + Clone + Send + Sync + Unpin + 'static,
    {
        let mut encoder = Encoder::encode(self);
        let mut writer = writer;
        let bytes_written = match io::copy(&mut encoder, &mut writer).await {
            Ok(b) => b,
            Err(err) => {
                // only log, don't break connection here. If connection is really closed, then the
                // next decode will break the loop receiving requests
                error!("Error sending response: {}", err);
                return Err(err);
            }
        };

        Ok(ResponseWritten { bytes_written })
    }
}

/// `ResponseWriter` has two responsibilities:
/// - Hold a `Response` which can be modified or replaced.
/// - Expose a `send` method which will immediately write the Response to the Http connection.
///
/// A `ResponseWriter` is initialized with a `Response` that contains:
/// - An empty body
/// - No headers (except that content-type defaults to `application/octet-stream` if not specified
/// and there's a body)`
/// - A 200 OK status
///
/// You can modify the `Response` as they see fit. Note, however, that a `Body` is not
/// necessarily in sync with the `content-type` headers that are sent. for example, it's possible
/// to set the Body using a string, and then set the content-type header on the Response to be
/// `content-type: video/mp4'. The power is in the your hands.
///
/// There are two convenience methods which will set the content-type:
/// - `set_text`, because there's no guess as to content-type, and
/// - `set_sse`, because the content-type `text/event-stream` is required.
///
/// If you wish to create a `Response` separately and then apply it to the `ResponseWriter`, you can
/// use `tophat::http::Response` and `tophat::Body`, and then `ReponseWriter::response_mut`.
///
/// All methods on `ResponseWriter` should list what headers they modify in the document string, and
/// the type of the parameter should be reflected in the function name (i.e. `text` takes a string,
/// not a stream or reader).
///
/// Possible body types:
/// - &str/String,
/// - AsyncRead,
/// - Stream (StreamExt),
pub struct ResponseWriter<W>
where
    W: AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    pub(crate) response: Response,
    pub(crate) writer: W,
}

impl<W> ResponseWriter<W>
where
    W: AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    /// send response, and return number of bytes written
    pub async fn send(self) -> Result<ResponseWritten, Glitch> {
        let (parts, body) = self.response.into_parts();

        let inner_resp = InnerResponse {
            status: parts.status,
            headers: parts.headers,
            version: parts.version,
            body,
        };

        Ok(inner_resp.send(self.writer).await?)
    }

    /// Sets response to specified code and immediately sends.
    ///
    /// Devised as a shortcut so it would be easier to send a response with an empty body and
    /// status code. But if body is present, it will send that. (There's no effect on anything
    /// besides the status code)
    ///
    /// Internally panics if status code is incorrect (use at your own risk! For something safer,
    /// try `set_status`.
    pub async fn send_code(self, code: u16) -> Result<ResponseWritten, Glitch> {
        let mut this = self;
        this.set_code(code);

        this.send().await
    }

    /// Set response to specified status_code.
    pub fn set_status(&mut self, status: http::StatusCode) -> &mut Self {
        *self.response.status_mut() = status;
        self
    }

    /// Set response to specified code.
    ///
    /// Internally panics if code is incorrect (use at your own risk! For something safer, try
    /// `set_status`.
    pub fn set_code(&mut self, code: u16) -> &mut Self {
        *self.response.status_mut() = http::StatusCode::from_u16(code).unwrap();
        self
    }

    /// Set response to specified body.
    ///
    /// Does not change content-type, that must be set separately in headers.
    pub fn set_body(&mut self, body: Body) -> &mut Self {
        *self.response.body_mut() = body;
        self
    }

    /// Append header to response. Will not replace a header with the same header name.
    pub fn append_header(
        &mut self,
        header_name: impl IntoHeaderName,
        header_value: HeaderValue,
    ) -> &mut Self {
        self.response
            .headers_mut()
            .append(header_name, header_value);
        self
    }

    /// Insert header to response. Replaces a header with the same header name.
    pub fn insert_header(
        &mut self,
        header_name: impl IntoHeaderName,
        header_value: HeaderValue,
    ) -> &mut Self {
        self.response
            .headers_mut()
            .insert(header_name, header_value);
        self
    }

    /// Mutable access to the full response. This way, if you like you can create the `Response`
    /// separately, and then set it in the `ResponseWriter`
    /// ```rust
    /// # use futures_util::io::{AsyncRead, AsyncWrite};
    /// # use std::error::Error;
    /// # use tophat::{Body, Request, Response, server::{glitch::Result, ResponseWriter, ResponseWritten}};
    /// async fn handler<W>(req: Request, mut resp_wtr: ResponseWriter<W>) -> Result<ResponseWritten>
    ///     where W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
    /// {
    ///     let resp = Response::new(Body::empty());
    ///     *resp_wtr.response_mut() = resp;
    ///     resp_wtr.send().await
    /// }
    /// ```
    pub fn response_mut(&mut self) -> &mut Response {
        &mut self.response
    }

    /// Retrieve a reference to the `Response` in the `ResponseWriter`
    pub fn response(&self) -> &Response {
        &self.response
    }

    /// Set response to:
    /// - 200 OK
    /// - Content-type text/plain
    /// - Body from String
    ///
    pub fn set_text(&mut self, text: String) -> &mut Self {
        *self.response.body_mut() = text.into();
        self.response
            .headers_mut()
            .insert(http::header::CONTENT_TYPE, "text/plain".parse().unwrap());
        self
    }

    /// Sets the response body as a Server Sent Events response stream.
    /// Adds the content-type header for SSE.
    ///
    /// Takes a `futures::Stream`, and `futures::TryStreamExt` must be in scope.
    pub fn set_sse<S>(&mut self, stream: S)
    where
        S: TryStreamExt<Error = std::io::Error> + Send + Sync + Unpin + 'static,
        S::Ok: AsRef<[u8]> + Send + Sync,
    {
        let stream = stream.into_async_read();

        self.set_body(Body::from_reader(stream, None));
        self.insert_header(http::header::CONTENT_TYPE, "text/event-stream".parse().unwrap());
    }
}

/// A marker to ensure that a response is written inside a request handler.
pub struct ResponseWritten {
    bytes_written: u64,
}

impl ResponseWritten {
    /// Bytes written by `ResponseWriter`
    pub fn bytes_written(&self) -> u64 {
        self.bytes_written
    }
}
