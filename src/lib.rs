// just the server for now

mod body;
mod decode;

use futures_core::Future;
use futures_io::{AsyncRead, AsyncWrite};
use http::Request as HttpRequest;

use crate::body::Body;
use crate::decode::decode;

type Request = HttpRequest<Body>;

struct Response<Body> {
    body: Body,
}

/// Accpet a new incoming Http/1.1 connection
pub async fn accept<RW, F, Fut>(addr: &str, io: RW, endpoint: F) -> http::Result<()>
where
    RW: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
    F: Fn(Request) -> Fut,
    Fut: Future<Output = http::Result<()>>,
{
    // first decode
    let req = decode(addr, io).await?.unwrap();

    endpoint(req).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_request() {
        smol::run(async {
            let testcase = TestCase;
            let sink = TestCase;

            let addr = "http://example.com";
            accept(addr, testcase, |req| async move {
                println!("hit");
                let body_bytes = req.body().as_bytes().unwrap().unwrap();
                let body = std::str::from_utf8(&*body_bytes).unwrap();
                println!("body: {}", body);
                panic!();
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
            buf: &mut [u8],
        ) -> Poll<io::Result<usize>> {
            let example = b"GET /foo/bar HTTP/1.1\r\nHost: example.org\r\nContent-Length: 7\r\n\r\nthebody\r\n".to_vec();
            let len = example.len();
            io::Read::read(&mut std::io::Cursor::new(example), buf).unwrap();
            Poll::Ready(Ok(len))
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
