use futures_io::AsyncWrite;
use http::Response as HttpResponse;

use crate::body::Body;
use crate::encode::Encoder;
use crate::error::Result;

/// Currently, Response is not generic over Body type
pub type Response = HttpResponse<Body>;

pin_project_lite::pin_project! {
    pub(crate) struct InnerResponse {
        // Currently just copying over the head
        pub(crate) head: HttpResponse<()>,
        #[pin]
        pub(crate)body: Body,
    }
}

pub struct ResponseWriter<RW>
where
    RW: AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    pub writer: RW,
}

impl<RW> ResponseWriter<RW>
where
    RW: AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    /// send response, and TODO return number of bytes written (I guess this would be a struct for more
    /// complicated sends, like with compression)
    pub async fn send(self, resp: Response) -> Result<ResponseWritten> {
        let mut writer = self.writer;

        let inner_resp = InnerResponse {
            head: HttpResponse::new(()), // just copy metadata over
            body: resp.into_body(),
        };
        let mut encoder = Encoder::encode(inner_resp);
        futures_util::io::copy(&mut encoder, &mut writer).await?;
        Ok(ResponseWritten)
    }
}

// is there a way to do a compile-time check here for whether resp_wtr.send() was called? Maybe
// by creating a new type from it.
// I guess the easy way is by a marker like ResponseWritten, which must be passed to the end of
// the handler. But is this too unwieldy? Shouldn't be too bad.
//
// TODO have a ReponseResult, which may contain bytes read etc. And then have it transform into
// ResponseWritten, to minimize boilerplate
pub struct ResponseWritten;
