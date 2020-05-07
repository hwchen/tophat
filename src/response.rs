use futures_io::AsyncWrite;
use http::{
    header::HeaderMap,
    status::StatusCode,
    version::Version,
    Response as HttpResponse,
};

use crate::body::Body;
use crate::encode::Encoder;
use crate::error::{Error, Result};

/// Currently, Response is not generic over Body type
pub type Response = HttpResponse<Body>;

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
    /// used for bad request in decoding.
    pub(crate) fn bad_request() -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            headers: HeaderMap::new(),
            version: Version::default(),
            body: Body::empty(),
        }
    }

    pub(crate) async fn send<W>(self, writer: W) -> Result<ResponseWritten>
        where W: AsyncWrite + Clone + Send + Sync + Unpin + 'static,
    {
        let mut encoder = Encoder::encode(self);
        let mut writer = writer;
        futures_util::io::copy(&mut encoder, &mut writer).await.map_err(|err| Error::Connection(err))?;
        Ok(ResponseWritten)
    }
}

pub struct ResponseWriter<W>
where
    W: AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    pub writer: W,
}

impl<W> ResponseWriter<W>
where
    W: AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    /// send response, and TODO return number of bytes written (I guess this would be a struct for more
    /// complicated sends, like with compression)
    pub async fn send(self, resp: Response) -> Result<ResponseWritten> {
        let (parts, body) = resp.into_parts();

        let inner_resp = InnerResponse {
            status: parts.status,
            headers: parts.headers,
            version: parts.version,
            body,
        };

        inner_resp.send(self.writer).await
    }
}

// TODO have a ReponseResult, which may contain bytes read etc. And then have it transform into
// ResponseWritten, to minimize boilerplate
pub struct ResponseWritten;

