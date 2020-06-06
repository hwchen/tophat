mod chunked_text_big;
mod mock;

use http::{
    header::{self, HeaderName, HeaderValue},
    method::Method,
    Uri, Version,
};
use tophat::{server::accept, Body};

use mock::{Cursor, Client};

const RESP_200: &str = "HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\n";
const RESP_400: &str = "HTTP/1.1 400 Bad Request\r\ncontent-length: 0\r\n\r\n";

#[test]
fn test_request_empty_body() {
    smol::block_on(async {
        let testclient = Client::new(
            "GET /foo/bar HTTP/1.1\r\nHost: example.org\r\n\r\n",
            RESP_200,
        );

        accept(testclient.clone(), |_req, resp_wtr| async move {
            // Won't compile if done is not returned in Ok!
            let done = resp_wtr.send().await.unwrap();

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
        let testclient = Client::new(
            "GET /foo/bar?one=two HTTP/1.1\r\nHost: example.org\r\nContent-Length: 6\r\n\r\ntophat",
            "HTTP/1.1 200 OK\r\ncontent-length: 12\r\n\r\nHello tophat",
        );

        accept(testclient.clone(), |req, mut resp_wtr| async move {
            // some basic parsing tests
            assert_eq!(req.uri().path(), Uri::from_static("/foo/bar"));
            assert_eq!(req.uri().query(), Some("one=two"));
            assert_eq!(req.version(), Version::HTTP_11);
            assert_eq!(req.method(), Method::GET);
            assert_eq!(
                req.headers().get(header::CONTENT_LENGTH),
                Some(&HeaderValue::from_bytes(b"6").unwrap())
            );
            assert_eq!(
                req.headers().get(header::HOST),
                Some(&HeaderValue::from_bytes(b"example.org").unwrap())
            );

            let body = req.into_body().into_string().await.unwrap();

            let res_body = format!("Hello {}", body);

            resp_wtr.set_body(res_body.into());
            let done = resp_wtr.send().await.unwrap();

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
        let testclient = Client::new(
            "/foo/bar HTTP/1.1\r\nHost: example.org\r\nContent-Length: 0\r\n\r\n",
            RESP_400,
        );

        accept(testclient.clone(), |_req, resp_wtr| async move {
            resp_wtr.send().await
        })
        .await
        .unwrap();

        testclient.assert();
    });
}

#[test]
fn test_request_missing_host() {
    smol::block_on(async {
        let testclient = Client::new(
            "GET /foo/bar HTTP/1.1\r\nContent-Length: 0\r\n\r\n",
            RESP_400,
        );

        accept(testclient.clone(), |_req, resp_wtr| async move {
            resp_wtr.send().await
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
        let testclient = Client::new(
            "GET /foo/bar HTTP/1.1\r\nHost: example.org\r\nContent-Length: 0\r\n\r\n",
            RESP_200,
        );

        accept(testclient.clone(), |req, resp_wtr| async move {
            assert_eq!(*req.uri(), Uri::from_static("/foo/bar"));
            assert_eq!(*req.uri().path(), Uri::from_static("/foo/bar"));
            resp_wtr.send().await
        })
        .await
        .unwrap();

        testclient.assert();

        // good absolute uri, ignores host
        let testclient = Client::new(
            "GET https://wunder.org/foo/bar HTTP/1.1\r\nHost: example.org\r\nContent-Length: 0\r\n\r\n",
            RESP_200,
        );

        accept(testclient.clone(), |req, resp_wtr| async move {
            assert_eq!(*req.uri(), Uri::from_static("https://wunder.org/foo/bar"));
            assert_eq!(*req.uri().path(), Uri::from_static("/foo/bar"));
            resp_wtr.send().await
        })
        .await
        .unwrap();

        testclient.assert();

        // bad uri path
        let testclient = Client::new(
            "GET foo/bar HTTP/1.1\r\nHost: example.org\r\nContent-Length: 0\r\n\r\n",
            RESP_400,
        );

        accept(testclient.clone(), |_req, resp_wtr| async move {
            resp_wtr.send().await
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
        let testclient = Client::new(
            "GET /foo/bar HTP/1.1\r\nHost: example.org\r\nContent-Length: 0\r\n\r\n",
            RESP_400,
        );

        accept(testclient.clone(), |_req, resp_wtr| async move {
            resp_wtr.send().await
        })
        .await
        .unwrap();

        testclient.assert();
    });

    // version 1.0 not supported
    smol::block_on(async {
        let testclient = Client::new(
            "GET /foo/bar HTTP/1.0\r\nHost: example.org\r\nContent-Length: 0\r\n\r\n",
            "HTTP/1.1 505 HTTP Version Not Supported\r\ncontent-length: 0\r\n\r\n",
        );

        accept(testclient.clone(), |_req, resp_wtr| async move {
            resp_wtr.send().await
        })
        .await
        .unwrap();

        testclient.assert();
    });
}

// sends message _ands_ closes connection
#[test]
fn test_transfer_encoding_unsupported() {
    smol::block_on(async {
        let testclient = Client::new(
            "GET /foo/bar HTTP/1.1\r\nHost: example.org\r\nContent-Length: 0\r\nTransfer-Encoding: gzip\r\n\r\n",
            "HTTP/1.1 501 Not Implemented\r\ncontent-length: 0\r\n\r\n",
        );

        let res = accept(testclient.clone(), |_req, resp_wtr| async move {
            resp_wtr.send().await
        })
        .await;

        match res {
            Ok(_) => panic!(),
            Err(err) => match err {
                tophat::server::ServerError::ConnectionClosedUnsupportedTransferEncoding => (),
                _ => panic!(),
            },
        }

        testclient.assert();
    });
}

#[test]
// TODO handle transfer-encoding chunked and content-length clash
#[ignore] // temporary
fn test_transfer_encoding_content_length() {
    smol::block_on(async {
        let testclient = Client::new(
            "GET /foo/bar HTTP/1.1\r\nHost: example.org\r\nContent-Length: 0\r\nTransfer-Encoding: chunked\r\n\r\n",
            RESP_400,
        );

        accept(testclient.clone(), |_req, resp_wtr| async move {
            resp_wtr.send().await
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
        let testclient = Client::new(
            "GET /foo/bar HTTP/1.1\r\nHost: example.org\r\nContent-Length: 0\r\n\r\n",
            RESP_200,
        );

        accept(testclient.clone(), |_req, mut resp_wtr| async move {
            resp_wtr.append_header(header::TRANSFER_ENCODING, "chunked".parse().unwrap());
            resp_wtr.send().await
        })
        .await
        .unwrap();

        testclient.assert();
    });

    smol::block_on(async {
        let testclient = Client::new(
            "GET /foo/bar HTTP/1.1\r\nHost: example.org\r\nContent-Length: 0\r\n\r\n",
            "HTTP/1.1 200 OK\r\ntransfer-encoding: chunked\r\n\r\n0\r\n\r\n",
        );

        accept(testclient.clone(), |_req, mut resp_wtr| async move {
            resp_wtr.set_body(Body::from_reader(Cursor::new(""), None));
            resp_wtr.append_header(header::CONTENT_LENGTH, "20".parse().unwrap());
            resp_wtr.send().await
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
        let testclient = Client::new(
            "GET /foo/bar HTTP/1.1\r\nHost: example.org\r\nContent-Length: 0\r\nTransfer-Encoding: chunked\r\n\r\n",
            RESP_200,
        );

        accept(testclient.clone(), |_req, mut resp_wtr| async move {
            resp_wtr.append_header(
                header::DATE,
                "Wed, 21 Oct 2015 07:28:00 GMT".parse().unwrap(),
            );
            resp_wtr.send().await
        })
        .await
        .unwrap();

        // One Date header should be stripped out by Client
        testclient.assert_with_resp_date("Wed, 21 Oct 2015 07:28:00 GMT");
    });
}

#[test]
fn test_set_content_type_mime() {
    smol::block_on(async {
        let testclient = Client::new(
            "GET /foo/bar HTTP/1.1\r\nHost: example.org\r\nContent-Length: 0\r\n\r\n",
            "HTTP/1.1 200 OK\r\ncontent-length: 0\r\ncontent-type: text/plain\r\n\r\n",
        );

        accept(testclient.clone(), |_req, mut resp_wtr| async move {
            resp_wtr.append_header(header::CONTENT_TYPE, "text/plain".parse().unwrap());
            resp_wtr.send().await
        })
        .await
        .unwrap();

        // One Date header should be stripped out by Client
        testclient.assert();
    });
}

#[test]
fn test_decode_transfer_encoding_chunked() {
    smol::block_on(async {
        let testclient = Client::new(
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
            let (body, trailer) = req.into_body().into_string_with_trailer().await.unwrap();

            let trailer = trailer.unwrap().unwrap();

            assert_eq!(body, "MozillaDeveloperNetwork");
            assert_eq!(
                trailer.headers.iter().collect::<Vec<_>>(),
                vec![(
                    &HeaderName::from_bytes(b"Expires").unwrap(),
                    &HeaderValue::from_bytes(b"Wed, 21 Oct 2015 07:28:00 GMT").unwrap(),
                )]
            );

            resp_wtr.send().await
        })
        .await
        .unwrap();

        testclient.assert();
    });

    // no trailer
    smol::block_on(async {
        let testclient = Client::new(
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
            let (body, trailer) = req.into_body().into_string_with_trailer().await.unwrap();

            let trailer = trailer.unwrap().unwrap();

            assert_eq!(body, "MozillaDeveloperNetwork");
            assert!(trailer.headers.is_empty());

            resp_wtr.send().await
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
        let testclient = Client::new_with_writes(
            "GET /foo/bar HTTP/1.1\r\nHost: example.org\r\nContent-Length: 0\r\n\r\n",
            "HTTP/1.1 200 OK\r\ntransfer-encoding: chunked\r\n\r\nD\r\nHello tophat!\r\n0\r\n\r\n",
            2,
        );

        accept(testclient.clone(), |_req, mut resp_wtr| async move {
            let body_str = Cursor::new("Hello tophat!");
            resp_wtr.set_body(Body::from_reader(body_str, None));

            resp_wtr.send().await
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
        let testclient = Client::new_with_writes(
            "GET /foo/bar HTTP/1.1\r\nHost: example.org\r\nContent-Length: 0\r\n\r\n",
            chunked_text_big::RESPONSE,
            1,
        );

        accept(testclient.clone(), |_req, mut resp_wtr| async move {
            let body_str = Cursor::new(chunked_text_big::TEXT);
            resp_wtr.set_body(Body::from_reader(body_str, None));

            resp_wtr.send().await
        })
        .await
        .unwrap();

        testclient.assert();
    });
}
