mod chunked_text_big;
mod test_client;
use test_client::Cursor;

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
fn test_dont_allow_user_set_body_type_header() {
    // Even if user sets the header for content-length or transfer-encoding, just ignore because
    // the encoding step will set it automatically
    //
    // Just test the two conflicting cases
    smol::block_on(async {
        let testclient = TestClient::new(
            "GET /foo/bar HTTP/1.1\r\nHost: example.org\r\nContent-Length: 0\r\n\r\n",
            RESP_200,
        );

        accept(testclient.clone(), |_req, resp_wtr| async move {
            let mut resp = HttpResponse::new(Body::empty());
            resp.headers_mut().append(header::TRANSFER_ENCODING, "chunked".parse().unwrap());
            resp_wtr.send(resp).await
        })
        .await
        .unwrap();

        testclient.assert();
    });

    smol::block_on(async {
        let testclient = TestClient::new(
            "GET /foo/bar HTTP/1.1\r\nHost: example.org\r\nContent-Length: 0\r\n\r\n",
            "HTTP/1.1 200 OK\r\ntransfer-encoding: chunked\r\n\r\n0\r\n\r\n",
        );

        accept(testclient.clone(), |_req, resp_wtr| async move {
            let mut resp = HttpResponse::new(Body::from_reader(Cursor::new(""), None));
            resp.headers_mut().append(header::CONTENT_LENGTH, "20".parse().unwrap());
            resp_wtr.send(resp).await
        })
        .await
        .unwrap();

        testclient.assert();
    });
}

#[test]
fn test_response_date() {
    // make sure that date isn't doubled if it's also set in response
    // also make sure that the date header was passed through
    smol::block_on(async {
        let testclient = TestClient::new(
            "GET /foo/bar HTTP/1.1\r\nHost: example.org\r\nContent-Length: 0\r\nTransfer-Encoding: chunked\r\n\r\n",
            RESP_200,
        );

        accept(testclient.clone(), |_req, resp_wtr| async move {
            let mut resp = HttpResponse::new(Body::empty());
            resp.headers_mut().append(header::DATE, "Wed, 21 Oct 2015 07:28:00 GMT".parse().unwrap());
            resp_wtr.send(resp).await
        })
        .await
        .unwrap();

        // One Date header should be stripped out by TestClient
        testclient.assert_with_resp_date("Wed, 21 Oct 2015 07:28:00 GMT");
    });
}

#[test]
fn test_set_content_type_mime() {
    smol::block_on(async {
        let testclient = TestClient::new(
            "GET /foo/bar HTTP/1.1\r\nHost: example.org\r\nContent-Length: 0\r\n\r\n",
            "HTTP/1.1 200 OK\r\ncontent-length: 0\r\ncontent-type: text/plain\r\n\r\n",
        );

        accept(testclient.clone(), |_req, resp_wtr| async move {
            let mut resp = HttpResponse::new(Body::empty());
            resp.headers_mut().append(header::CONTENT_TYPE, tophat::mime::TEXT_PLAIN.to_string().parse().unwrap());
            resp_wtr.send(resp).await
        })
        .await
        .unwrap();

        // One Date header should be stripped out by TestClient
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

#[test]
fn test_encode_transfer_encoding_chunked() {
    smol::block_on(async {
        // 13 is D in hexadecimal.
        // Need two writes because there's a chunk and then there's the end.
        let testclient = TestClient::new_with_writes(
            "GET /foo/bar HTTP/1.1\r\nHost: example.org\r\nContent-Length: 0\r\n\r\n",
            "HTTP/1.1 200 OK\r\ntransfer-encoding: chunked\r\n\r\nD\r\nHello tophat!\r\n0\r\n\r\n",
            2,
        );

        accept(testclient.clone(), |_req, resp_wtr| async move {
            let body_str = Cursor::new("Hello tophat!");
            let body = Body::from_reader(body_str, None);

            let resp = HttpResponse::new(body);
            resp_wtr.send(resp).await
        })
        .await
        .unwrap();

        testclient.assert();
    });
}

#[test]
#[ignore] // TODO figure out how to line up chunks
fn test_encode_transfer_encoding_chunked_big() {
    smol::block_on(async {
        // 13 is D in hexadecimal.
        // Need two writes because there's a chunk and then there's the end.
        let testclient = TestClient::new_with_writes(
            "GET /foo/bar HTTP/1.1\r\nHost: example.org\r\nContent-Length: 0\r\n\r\n",
            chunked_text_big::RESPONSE,
            1,
        );

        accept(testclient.clone(), |_req, resp_wtr| async move {
            let body_str = Cursor::new(chunked_text_big::TEXT);
            let body = Body::from_reader(body_str, None);

            let resp = HttpResponse::new(body);
            resp_wtr.send(resp).await
        })
        .await
        .unwrap();

        testclient.assert();
    });
}
