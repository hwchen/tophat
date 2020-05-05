use futures_io::AsyncWrite;
use http::Response as HttpResponse;

use crate::body::Body;
use crate::encode::Encoder;

/// Currently, Response is not generic over Body type
type Response = HttpResponse<Body>;

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
    // TODO try #[must_use] here
    /// send response, and return number of bytes written (I guess this would be a struct for more
    /// complicated sends, like with compression)
    pub async fn send(self, resp: Response) -> http::Result<ResponseWritten> {
        let mut writer = self.writer;
        let mut encoder = Encoder::encode(resp.body());
        futures_util::io::copy(&mut encoder, &mut writer).await.unwrap();
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
