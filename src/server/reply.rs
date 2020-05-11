//! Convenience methods for createing a response.

use futures_util::TryStreamExt;

use crate::Response;
use crate::body::Body;

/// Turn a stream into a Server Sent Events response stream.
///
/// Takes a `futures::Stream`, and `futures::TryStreamExt` must be in scope.
pub fn sse<S: 'static>(stream: S) -> Response
    where S: TryStreamExt<Error = std::io::Error> + Send + Sync + Unpin,
        S::Ok: AsRef<[u8]> + Send + Sync,
{
    let stream = stream.into_async_read();

    let body = Body::from_reader(stream, None);
    let mut resp = Response::new(body);
    resp.headers_mut().insert(
        "content-type",
        "text/event-stream".parse().unwrap(),
    );

    resp
}
