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
use thiserror::Error as ThisError;

use crate::Request;
use crate::body::Body;
use crate::response::InnerResponse;

const LF: u8 = b'\n';

/// Decode and http request
///
/// Errors are bubbled up and handled in `accept`, the possible decode errors and the error handler
/// are defined in this module.
///
/// `None` means that no request was read.
pub(crate) async fn decode<R>(addr: &str, reader: R) -> Result<Option<Request>, DecodeFail>
where
    R: AsyncRead + Unpin + Send + Sync + 'static
{
    use DecodeFail::*;

    let mut reader = BufReader::new(reader);
    let mut buf = Vec::new();
    let mut headers = [httparse::EMPTY_HEADER; 16];
    let mut httparse_req = httparse::Request::new(&mut headers);

    // Keep reading bytes from the stream until we hit the end of the head.
    loop {
        let bytes_read = reader.read_until(LF, &mut buf).await.map_err(|err| ConnectionLost(err))?;

        // No bytes read, no request.
        if bytes_read == 0 {
            return Ok(None);
        }

        // We've hit the end delimiter of the head.
        let idx = buf.len() - 1;
        if idx >= 3 && &buf[idx - 3..=idx] == b"\r\n\r\n" {
            break;
        }
    }

    // Convert our header buf into an httparse instance, and validate.
    let status = httparse_req.parse(&buf)?;

    // TODO error type
    if status.is_partial() { return Err(HttpMalformedHead) };


    // TODO remove allocation
    // TODO use host header?
    let path = httparse_req.path.ok_or(HttpNoPath)?;
    let uri: Uri = format!("{}{}", addr, path).parse()?;

    let method = http::Method::from_bytes(httparse_req.method.ok_or(HttpNoMethod)?.as_bytes())?;
    let version = if httparse_req.version.ok_or(HttpNoVersion)? == 1 {
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
            content_length = Some(
                std::str::from_utf8(header.value)
                .map_err(|_| HttpInvalidContentLength)?
                .parse::<usize>()
                .map_err(|_| HttpInvalidContentLength)?
            );
        }

        req.headers_mut().expect("Request builder error")
            .append(
                HeaderName::from_bytes(header.name.as_bytes())?,
                HeaderValue::from_bytes(header.value)?
            );
    }


    // Handling content-length v. transfer-encoding:
    // https://tools.ietf.org/html/rfc7230#section-3.3.3
    let content_length = content_length.unwrap_or(0);

    let body = reader.take(content_length as u64);
    let req = req
        .body(Body::from_reader(body, Some(content_length)))
        .map_err(|_| HttpRequestBuild)?;

    Ok(Some(req))
}

#[derive(ThisError, Debug)]
pub(crate) enum DecodeFail {
    #[error("Connection Lost: {0}")]
    ConnectionLost(std::io::Error),
    #[error("Http parse malformed head")]
    HttpMalformedHead,

    // TODO check that these are actually errors, and not just something to handle
    #[error("Http no path found")]
    HttpNoPath,
    #[error("Http no method found")]
    HttpNoMethod,
    #[error("Http: no version found")]
    HttpNoVersion,
    #[error("Http invalid content length")]
    HttpInvalidContentLength,
    #[error("Http request could not be built")]
    HttpRequestBuild,

    // conversions related to http and httparse lib
    #[error("Http header parsing error: {0}")]
    HeaderParse(#[from] httparse::Error),
    #[error("Http Uri error: {0}")]
    HttpUri(#[from] http::uri::InvalidUri),
    #[error("Http Method error: {0}")]
    HttpMethod(#[from] http::method::InvalidMethod),
    #[error("Http Header name error: {0}")]
    HttpHeaderName(#[from] http::header::InvalidHeaderName),
    #[error("Http Header value error: {0}")]
    HttpHeaderValue(#[from] http::header::InvalidHeaderValue),
}

pub(crate) fn fail_to_response_and_log(fail: DecodeFail) -> Option<InnerResponse> {
    use log::*;
    use DecodeFail::*;

    // TODO improve logging message
    debug!("Decode error: {} ", fail);

    match fail {
        ConnectionLost(_) => None,
        _ => Some(InnerResponse::bad_request()),
    }
}
