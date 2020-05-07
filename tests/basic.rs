mod test_client;

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

use test_client::TestClient;

#[test]
fn test_empty_body() {
    smol::block_on(async {
        let testclient = TestClient::new(
            "GET /foo/bar HTTP/1.1\r\nHost: example.org\r\n\r\n",
            "HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\n"
        );

        let addr = "http://example.org";
        accept(addr, testclient.clone(), |_req, resp_wtr| async move {
            println!("hit");
            let resp = HttpResponse::new(Body::empty());
            // Won't compile if done is not returned in Ok!
            let done = resp_wtr.send(resp).await.unwrap();

            Ok(done)
        })
        .await
        .unwrap();

        testclient.assert();
    });
}

//#[test]
//fn test_basic_request() {
//    smol::block_on(async {
//        let testcase = TestClient { write_buf: Arc::new(Mutex::new(vec![])) };
//        let example = b"GET /foo/bar HTTP/1.1\r\nHost: example.org\r\nContent-Length: 6\r\n\r\ntophat".to_vec();
//
//        let addr = "http://example.org";
//        accept(addr, testcase.clone(), |req, resp_wtr| async move {
//            // some basic parsing tests
//            assert_eq!(*req.uri(), Uri::from_static("http://example.org/foo/bar"));
//            assert_eq!(req.version(), Version::HTTP_11);
//            assert_eq!(req.method(), Method::GET);
//            assert_eq!(req.headers().get(header::CONTENT_LENGTH), Some(&HeaderValue::from_bytes(b"6").unwrap()));
//            assert_eq!(req.headers().get(header::HOST), Some(&HeaderValue::from_bytes(b"example.org").unwrap()));
//
//            let body = req.into_body().into_string().await.unwrap();
//
//            let res_body = format!("Hello {}", body);
//
//            let resp = HttpResponse::new(res_body.into());
//            let done = resp_wtr.send(resp).await.unwrap();
//
//            Ok(done)
//        })
//        .await
//        .unwrap();
//
//        assert_eq!(testcase.out_string(), "Hello tophat".to_owned())
//    });
//}
