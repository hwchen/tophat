mod test_client;

use http::Response as HttpResponse;
use http::{
    header::{
        self,
        HeaderName,
        HeaderValue,
    },
    method::Method,
    Version,
    Uri,
};
use tophat::{accept, Body};

use test_client::TestClient;

const RESP_200: &str = "HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\n";
const RESP_400: &str = "HTTP/1.1 400 Bad Request\r\ncontent-length: 0\r\n\r\n";

#[test]
fn test_request_empty_body() {
    smol::block_on(async {
        let testclient = TestClient::new(
            "GET /foo/bar HTTP/1.1\r\nHost: example.org\r\n\r\n",
            RESP_200,
        );

        accept(testclient.clone(), |_req, resp_wtr| async move {
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

#[test]
fn test_request_basic_with_body_and_query() {
    smol::block_on(async {
        let testclient = TestClient::new(
            "GET /foo/bar?one=two HTTP/1.1\r\nHost: example.org\r\nContent-Length: 6\r\n\r\ntophat",
            "HTTP/1.1 200 OK\r\ncontent-length: 12\r\n\r\nHello tophat",
        );

        accept(testclient.clone(), |req, resp_wtr| async move {
            // some basic parsing tests
            assert_eq!(req.uri().path(), Uri::from_static("/foo/bar"));
            assert_eq!(req.uri().query(), Some("one=two"));
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

        testclient.assert();
    });
}
#[test]
fn test_request_missing_method() {
    smol::block_on(async {
        let testclient = TestClient::new(
            "/foo/bar HTTP/1.1\r\nHost: example.org\r\nContent-Length: 0\r\n\r\n",
            RESP_400,
        );

        accept(testclient.clone(), |_req, resp_wtr| async move {
            let resp = HttpResponse::new(Body::empty());
            resp_wtr.send(resp).await
        })
        .await
        .unwrap();

        testclient.assert();
    });
}

#[test]
fn test_request_missing_host() {
    smol::block_on(async {
        let testclient = TestClient::new(
            "GET /foo/bar HTTP/1.1\r\nContent-Length: 0\r\n\r\n",
            RESP_400,
        );

        accept(testclient.clone(), |_req, resp_wtr| async move {
            let resp = HttpResponse::new(Body::empty());
            resp_wtr.send(resp).await
        })
        .await
        .unwrap();

        testclient.assert();
    });
}

#[test]
// ignore host, should return abs_path or AbsoluteURI from uri
fn test_request_path() {
    smol::block_on(async {
        // good uri path
        let testclient = TestClient::new(
            "GET /foo/bar HTTP/1.1\r\nHost: example.org\r\nContent-Length: 0\r\n\r\n",
            RESP_200,
        );

        accept(testclient.clone(), |req, resp_wtr| async move {
            assert_eq!(*req.uri(), Uri::from_static("/foo/bar"));
            assert_eq!(*req.uri().path(), Uri::from_static("/foo/bar"));
            let resp = HttpResponse::new(Body::empty());
            resp_wtr.send(resp).await
        })
        .await
        .unwrap();

        testclient.assert();

        // good absolute uri, ignores host
        let testclient = TestClient::new(
            "GET https://wunder.org/foo/bar HTTP/1.1\r\nHost: example.org\r\nContent-Length: 0\r\n\r\n",
            RESP_200,
        );

        accept(testclient.clone(), |req, resp_wtr| async move {
            assert_eq!(*req.uri(), Uri::from_static("https://wunder.org/foo/bar"));
            assert_eq!(*req.uri().path(), Uri::from_static("/foo/bar"));
            let resp = HttpResponse::new(Body::empty());
            resp_wtr.send(resp).await
        })
        .await
        .unwrap();

        testclient.assert();

        // bad uri path
        let testclient = TestClient::new(
            "GET foo/bar HTTP/1.1\r\nHost: example.org\r\nContent-Length: 0\r\n\r\n",
            RESP_400,
        );

        accept(testclient.clone(), |_req, resp_wtr| async move {
            let resp = HttpResponse::new(Body::empty());
            resp_wtr.send(resp).await
        })
        .await
        .unwrap();

        testclient.assert();
    });
}

#[test]
fn test_request_version() {
    // malformed version
    smol::block_on(async {
        let testclient = TestClient::new(
            "GET /foo/bar HTP/1.1\r\nHost: example.org\r\nContent-Length: 0\r\n\r\n",
            RESP_400,
        );

        accept(testclient.clone(), |_req, resp_wtr| async move {
            let resp = HttpResponse::new(Body::empty());
            resp_wtr.send(resp).await
        })
        .await
        .unwrap();

        testclient.assert();
    });

    // version 1.0 not supported
    smol::block_on(async {
        let testclient = TestClient::new(
            "GET /foo/bar HTTP/1.0\r\nHost: example.org\r\nContent-Length: 0\r\n\r\n",
            "HTTP/1.1 505 HTTP Version Not Supported\r\ncontent-length: 0\r\n\r\n",
        );

        accept(testclient.clone(), |_req, resp_wtr| async move {
            let resp = HttpResponse::new(Body::empty());
            resp_wtr.send(resp).await
        })
        .await
        .unwrap();

        testclient.assert();
    });
}

#[test]
// TODO handle transfer-encoding chunked and content-length clash
#[ignore] // temporary
fn test_transfer_encoding_content_length() {
    smol::block_on(async {
        let testclient = TestClient::new(
            "GET /foo/bar HTTP/1.1\r\nHost: example.org\r\nContent-Length: 0\r\nTransfer-Encoding: chunked\r\n\r\n",
            RESP_400,
        );

        accept(testclient.clone(), |_req, resp_wtr| async move {
            let resp = HttpResponse::new(Body::empty());
            resp_wtr.send(resp).await
        })
        .await
        .unwrap();

        testclient.assert();
    });
}

#[test]
fn test_decode_transfer_encoding_chunked() {
    smol::block_on(async {
        let testclient = TestClient::new(
            "GET /foo/bar HTTP/1.1\r\nHost: example.org\r\nTransfer-Encoding: chunked\r\n\r\n\
                7\r\n\
                Mozilla\r\n\
                9\r\n\
                Developer\r\n\
                7\r\n\
                Network\r\n\
                0\r\n\
                Expires: Wed, 21 Oct 2015 07:28:00 GMT\r\n\
                \r\n",
            RESP_200,
        );

        accept(testclient.clone(), |req, resp_wtr| async move {
            // If you want to wait for trailer, need to use this method.
            // Reading body and trailer separately will run into borrow errors
            let (body, trailer) = req.into_body()
                .into_string_with_trailer()
                .await
                .unwrap();

            let trailer = trailer.unwrap().unwrap();

            assert_eq!(body, "MozillaDeveloperNetwork");
            assert_eq!(
                trailer.headers.iter().collect::<Vec<_>>(),
                vec![
                    (&HeaderName::from_bytes(b"Expires").unwrap(),
                    &HeaderValue::from_bytes(b"Wed, 21 Oct 2015 07:28:00 GMT").unwrap(),
                )]
            );

            let resp = HttpResponse::new(Body::empty());
            resp_wtr.send(resp).await
        })
        .await
        .unwrap();

        testclient.assert();
    });

    // no trailer
    smol::block_on(async {
        let testclient = TestClient::new(
            "GET /foo/bar HTTP/1.1\r\nHost: example.org\r\nTransfer-Encoding: chunked\r\n\r\n\
                7\r\n\
                Mozilla\r\n\
                9\r\n\
                Developer\r\n\
                7\r\n\
                Network\r\n\
                0\r\n\
                \r\n",
            RESP_200,
        );

        accept(testclient.clone(), |req, resp_wtr| async move {
            // If you want to wait for trailer, need to use this method.
            // Reading body and trailer separately will run into borrow errors
            let (body, trailer) = req.into_body()
                .into_string_with_trailer()
                .await
                .unwrap();

            let trailer = trailer.unwrap().unwrap();

            assert_eq!(body, "MozillaDeveloperNetwork");
            assert!(trailer.headers.is_empty());

            let resp = HttpResponse::new(Body::empty());
            resp_wtr.send(resp).await
        })
        .await
        .unwrap();

        testclient.assert();
    });
}
