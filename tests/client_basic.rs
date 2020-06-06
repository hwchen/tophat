//! Client tests are far from comprehensive; more tests are welcome.

mod mock;

use http::{ header, StatusCode, Uri};
use tophat::{client::connect, Body, Request};

use mock::Server;

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

#[test]
fn test_client_bad_request() {
    smol::block_on(async {
        // expected req, and sends a resp200
        let testserver = Server::new_with_writes(
            "GET /foo/bar HTTP/1.1\r\ncontent-length: 0\r\nhost: example.org\r\n\r\n",
            RESP_400,
            1,
        );

        let mut req = Request::new(Body::empty());
        // TODO make Host compile time error?
        req.headers_mut().insert(header::HOST, "example.org".parse().unwrap());
        *req.uri_mut() = "/foo/bar".parse::<Uri>().unwrap();

        let resp = connect(testserver.clone(), req).await.unwrap();

        testserver.assert();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    });
}

#[test]
fn test_client_body_query() {
    smol::block_on(async {
        // expected req, and sends a resp200
        let testserver = Server::new_with_writes(
            "GET /foo/bar?one=two HTTP/1.1\r\ncontent-length: 6\r\nhost: example.org\r\n\r\ntophat",
            RESP_200,
            1,
        );

        let mut req = Request::new("tophat".into());
        // TODO make Host compile time error?
        req.headers_mut().insert(header::HOST, "example.org".parse().unwrap());
        *req.uri_mut() = "/foo/bar?one=two".parse::<Uri>().unwrap();

        let resp = connect(testserver.clone(), req).await.unwrap();

        testserver.assert();
        assert_eq!(resp.status(), StatusCode::OK);
    });
}
