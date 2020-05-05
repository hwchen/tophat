// just the server for now

mod body;
mod decode;
mod encode;

use futures_core::Future;
use futures_io::{AsyncRead, AsyncWrite};
use http::Request as HttpRequest;

use crate::body::Body;
use crate::decode::decode;
use crate::encode::Encoder;

type Request = HttpRequest<Body>;

pub struct Response<RW>
where
    RW: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    pub body: Option<Body>,
    pub writer: RW,
}

impl<RW> Response<RW>
where
    RW: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    pub fn with_body(&mut self, body: Body) {
        self.body = Some(body)
    }

    /// send response, and return number of bytes written (I guess this would be a struct for more
    /// complicated sends, like with compression)
    pub async fn send(self) -> http::Result<usize> {
        let mut writer = self.writer;
        let mut encoder = Encoder::encode(self.body.unwrap());
        futures_util::io::copy(&mut encoder, &mut writer).await.unwrap();
        Ok(0)
    }
}

/// Accpet a new incoming Http/1.1 connection
pub async fn accept<RW, F, Fut>(addr: &str, io: RW, endpoint: F) -> http::Result<()>
where
    RW: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
    F: Fn(Request, Response<RW>) -> Fut,
    Fut: Future<Output = http::Result<()>>,
{
    // first decode
    let req = decode(addr, io.clone()).await?.unwrap();
    let resp = Response { body: None, writer: io };
    endpoint(req, resp).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use bytes::Bytes;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_basic_request() {
        smol::block_on(async {
            let testcase = TestCase { times_written: 0, write_buf: Arc::new(Mutex::new(vec![])) };

            let addr = "http://example.com";
            accept(addr, testcase.clone(), |req, mut res| async move {
                let body_bytes = req.body().as_bytes().unwrap().unwrap();
                let body = std::str::from_utf8(&*body_bytes).unwrap();
                let res_body = format!("Hello {}", body);

                res.with_body(Body::new(Bytes::from(res_body.into_bytes())));
                res.send().await.unwrap();

                Ok(())
            })
            .await
            .unwrap();

            assert_eq!(testcase.out_string(), "Hello tophat".to_owned())
        });
    }

    use std::io;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    #[derive(Clone)]
    struct TestCase {
        times_written: usize,
        write_buf: std::sync::Arc<Mutex<Vec<u8>>>,
    }

    impl TestCase {
        pub fn out_string(&self) -> String {
            let write_buf = self.write_buf.lock().unwrap();
            let write_buf = write_buf.clone();
            String::from_utf8(write_buf).unwrap()
        }
    }

    impl AsyncRead for TestCase {
        fn poll_read(
            self: Pin<&mut Self>,
            _cx: &mut Context,
            buf: &mut [u8],
        ) -> Poll<io::Result<usize>> {
            let example = b"GET /foo/bar HTTP/1.1\r\nHost: example.org\r\nContent-Length: 6\r\n\r\ntophat".to_vec();
            let len = example.len();
            io::Read::read(&mut std::io::Cursor::new(example), buf).unwrap();
            Poll::Ready(Ok(len))
        }
    }

    impl AsyncWrite for TestCase {
        fn poll_write(self: Pin<&mut Self>, _cx: &mut Context, buf: &[u8]) -> Poll<io::Result<usize>> {
            if self.times_written == 0 {
                let mut write_buf = self.write_buf.lock().unwrap();
                write_buf.extend_from_slice(buf);

                //dbg!(&buf);
                //dbg!(&write_buf);

                //self.times_written += 1;
                Poll::Ready(Ok(write_buf.len()))
            }  else {
                Poll::Pending
            }
        }

        fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<io::Result<()>> {
            Poll::Ready(Ok(())) // placeholder, shouldn't hit?
        }

        fn poll_close(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<io::Result<()>> {
            Poll::Ready(Ok(())) // placeholder, shouldn't hit?
        }
    }
}
