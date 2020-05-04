// just the server for now

mod body;

use futures_core::Future;
use futures_io::{AsyncRead, AsyncWrite};
use http::Request as HttpRequest;

use crate::body::Body;

type Request = HttpRequest<Body>;

/// Accpet a new incoming Http/1.1 connection
pub async fn accept<RW, F, Fut>(addr: &str, io: RW, endpoint: F) -> http::Result<()>
where
    RW: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
    F: Fn(Request) -> Fut,
    Fut: Future<Output = http::Result<()>>,
{
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_request() {
        smol::block_on(async {
            let testcase = TestCase;

            let addr = "http://example.com";
            accept(addr, testcase, |_req| async {
                Ok(())
            })
            .await
            .unwrap();
        });
    }

    use std::io;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    #[derive(Clone)]
    struct TestCase;

    impl AsyncRead for TestCase {
        fn poll_read(
            self: Pin<&mut Self>,
            _cx: &mut Context,
            _buf: &mut [u8],
        ) -> Poll<io::Result<usize>> {
            Poll::Ready(Ok(0))
        }
    }

    impl AsyncWrite for TestCase {
        fn poll_write(self: Pin<&mut Self>, _cx: &mut Context, _buf: &[u8]) -> Poll<io::Result<usize>> {
            Poll::Ready(Ok(0))
        }

        fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<io::Result<()>> {
            Poll::Ready(Ok(()))
        }

        fn poll_close(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<io::Result<()>> {
            Poll::Ready(Ok(()))
        }
    }
}
