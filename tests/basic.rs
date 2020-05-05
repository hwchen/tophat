use std::sync::{Arc, Mutex};
use http::Response as HttpResponse;
use http::{
    header::{
        self,
        HeaderValue,
    },
    method::Method,
    Version,
    Uri,
};

use tophat::{accept, Body};

#[test]
#[ignore]
fn test_empty_body() {
    smol::block_on(async {
        let testcase = TestCase { write_buf: Arc::new(Mutex::new(vec![])) };

        let addr = "http://example.com";
        accept(addr, testcase.clone(), |_req, resp_wtr| async move {
            let resp = HttpResponse::new(Body::empty());
            // Won't compile if done is not returned in Ok!
            let done = resp_wtr.send(resp).await.unwrap();

            Ok(done)
        })
        .await
        .unwrap();
    });
}

#[test]
fn test_basic_request() {
    smol::block_on(async {
        let testcase = TestCase { write_buf: Arc::new(Mutex::new(vec![])) };

        let addr = "http://example.org";
        accept(addr, testcase.clone(), |req, resp_wtr| async move {
            // some basic parsing tests
            assert_eq!(*req.uri(), Uri::from_static("http://example.org/foo/bar"));
            assert_eq!(req.version(), Version::HTTP_11);
            assert_eq!(req.method(), Method::GET);
            assert_eq!(req.headers().get(header::CONTENT_LENGTH), Some(&HeaderValue::from_bytes(b"6").unwrap()));
            assert_eq!(req.headers().get(header::HOST), Some(&HeaderValue::from_bytes(b"example.org").unwrap()));

            let body = req.into_body().into_string().await.unwrap();

            let res_body = format!("Hello {}", body);

            let resp = HttpResponse::new(res_body.into());
            let done = resp_wtr.send(resp).await.unwrap();

            Ok(done)
        })
        .await
        .unwrap();

        assert_eq!(testcase.out_string(), "Hello tophat".to_owned())
    });
}

// testing framework
use futures_io::{AsyncRead, AsyncWrite};
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Clone)]
struct TestCase {
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
        let mut write_buf = self.write_buf.lock().unwrap();
        write_buf.extend_from_slice(buf);

        //dbg!(&buf);
        //dbg!(&write_buf);
        Poll::Ready(Ok(write_buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(())) // placeholder, shouldn't hit?
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(())) // placeholder, shouldn't hit?
    }
}
