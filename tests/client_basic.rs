mod mock;

use http::{
    header::{self, HeaderName, HeaderValue},
    method::Method,
    StatusCode, Uri, Version,
};
use tophat::{client::connect, Body, Request};

use mock::{Cursor, Server};

const RESP_200: &str = "HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\n";
const RESP_400: &str = "HTTP/1.1 400 Bad Request\r\ncontent-length: 0\r\n\r\n";

#[test]
fn test_client_empties() {
    smol::block_on(async {
        // expected req, and sends a resp200
        let testserver = Server::new_with_writes(
            "GET /foo/bar HTTP/1.1\r\ncontent-length: 0\r\nhost: example.org\r\n\r\n",
            RESP_200,
            1,
        );

        let mut req = Request::new(Body::empty());
        // TODO make Host compile time error?
        req.headers_mut().insert(header::HOST, "example.org".parse().unwrap());
        *req.uri_mut() = "/foo/bar".parse::<Uri>().unwrap();

        let resp = connect(testserver.clone(), req).await.unwrap();

        testserver.assert();
        assert_eq!(resp.status(), StatusCode::OK);
    });
}

//#[test]
//fn test_request_basic_with_body_and_query() {
//    smol::block_on(async {
//        let testclient = Server::new(
//            "GET /foo/bar?one=two HTTP/1.1\r\nHost: example.org\r\nContent-Length: 6\r\n\r\ntophat",
//            "HTTP/1.1 200 OK\r\ncontent-length: 12\r\n\r\nHello tophat",
//        );
//
//        accept(testclient.clone(), |req, mut resp_wtr| async move {
//            // some basic parsing tests
//            assert_eq!(req.uri().path(), Uri::from_static("/foo/bar"));
//            assert_eq!(req.uri().query(), Some("one=two"));
//            assert_eq!(req.version(), Version::HTTP_11);
//            assert_eq!(req.method(), Method::GET);
//            assert_eq!(
//                req.headers().get(header::CONTENT_LENGTH),
//                Some(&HeaderValue::from_bytes(b"6").unwrap())
//            );
//            assert_eq!(
//                req.headers().get(header::HOST),
//                Some(&HeaderValue::from_bytes(b"example.org").unwrap())
//            );
//
//            let body = req.into_body().into_string().await.unwrap();
//
//            let res_body = format!("Hello {}", body);
//
//            resp_wtr.set_body(res_body.into());
//            let done = resp_wtr.send().await.unwrap();
//
//            Ok(done)
//        })
//        .await
//        .unwrap();
//
//        testclient.assert();
//    });
//}
