// TODO Handle all of the headers. See hyper src/proto/h1/role.rs
// - transfer encoding
// - connection
// - expect
// - upgrade
// etc.

use futures_io::AsyncRead;
use futures_util::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use http::header::{HeaderName, HeaderValue, CONTENT_LENGTH};
use http::uri::Uri;

use crate::body::Body;
use crate::{Result, Request};

const LF: u8 = b'\n';

pub(crate) async fn decode<R>(addr: &str, reader: R) -> Result<Option<Request>>
where
    R: AsyncRead + Unpin + Send + Sync + 'static
{
    let mut reader = BufReader::new(reader);
    let mut buf = Vec::new();
    let mut headers = [httparse::EMPTY_HEADER; 16];
    let mut httparse_req = httparse::Request::new(&mut headers);

    // Keep reading bytes from the stream until we hit the end of the stream.
    loop {
        let bytes_read = reader.read_until(LF, &mut buf).await?;

        // No more bytes are yielded from the stream.
        if bytes_read == 0 {
            return Ok(None);
        }

        // We've hit the end delimiter of the stream.
        let idx = buf.len() - 1;
        if idx >= 3 && &buf[idx - 3..=idx] == b"\r\n\r\n" {
            break;
        }
    }

    // Convert our header buf into an httparse instance, and validate.
    let status = httparse_req.parse(&buf).expect("Error parsing http header");

    // TODO error type
    if status.is_partial() { panic!("Malformed Header") }


    // TODO remove allocation
    let path = httparse_req.path.expect("no path");
    let uri: Uri = format!("{}{}", addr, path).parse().expect("error parsing uri");

    let method = http::Method::from_bytes(httparse_req.method.expect("no method").as_bytes()).expect("error parsing method from bytes");
    let version = if httparse_req.version.expect("no version") == 1 {
        //TODO keep_alive = true, is_http_11 = true
        http::Version::HTTP_11
    } else {
        //TODO keep_alive = false, is_http_11 = false
        http::Version::HTTP_10
    };

    let mut req = http::request::Builder::new()
        .method(method)
        .uri(uri)
        .version(version);


    // append headers
    // just check for content length for now
    // TODO check hyper for all the subtleties
    let mut content_length = None;
    for header in httparse_req.headers.iter() {
        if header.name == CONTENT_LENGTH {
            content_length = Some(std::str::from_utf8(header.value).expect("not utf8").parse::<usize>().expect("parse error"));
        }

        req.headers_mut()
            .expect("error getting headers_mut")
            .append(
                HeaderName::from_bytes(header.name.as_bytes()).expect("HeaderName parse error"),
                HeaderValue::from_bytes(header.value).expect("headervalue parse error")
            );
    }


    // TODO fix this when transfer encoding is allowed
    let content_length = content_length.unwrap_or(0);
    //dbg!(content_length);

    let body = reader.take(content_length as u64);
    let req = req
        .body(Body::from_reader(body, Some(content_length)))
        .expect("Error making body");

    Ok(Some(req))
}
