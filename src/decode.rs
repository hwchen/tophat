// TODO Handle all of the headers. See hyper src/proto/h1/role.rs
// - transfer encoding
// - connection
// - expect
// - upgrade
// etc.

use futures_io::AsyncRead;
use futures_util::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use http::header::{self, HeaderName, HeaderValue};
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
pub(crate) async fn decode<R>(reader: R) -> Result<Option<Request>, DecodeFail>
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
        let bytes_read = reader.read_until(LF, &mut buf).await.map_err(ConnectionLost)?;

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

    // Convert head buf into an httparse instance, and validate.
    let status = httparse_req.parse(&buf)?;
    if status.is_partial() { return Err(HttpMalformedHead) };

    // Check that req basics are here
    let method = http::Method::from_bytes(httparse_req.method.ok_or(HttpNoMethod)?.as_bytes())?;
    let version = if httparse_req.version.ok_or(HttpNoVersion)? == 1 {
        //TODO keep_alive = true, is_http_11 = true
        http::Version::HTTP_11
    } else {
        //TODO keep_alive = false, is_http_11 = false
        //http::Version::HTTP_10
        return Err(Http10NotSupported);
    };

    // Start with the basic request build, so we can add headers directly.
    let mut req = http::request::Builder::new();

    // Now check headers for special cases (e.g. content-length, host), and append all headers
    // TODO check hyper for all the subtleties
    let mut content_length = None;
    let mut has_host = false;
    #[allow(clippy::borrow_interior_mutable_const)] // TODO see if I can remove this later
    for header in httparse_req.headers.iter() {
        if header.name == header::CONTENT_LENGTH {
            content_length = Some(
                std::str::from_utf8(header.value)
                .map_err(|_| HttpInvalidContentLength)?
                .parse::<usize>()
                .map_err(|_| HttpInvalidContentLength)?
            );
        } else if header.name == header::TRANSFER_ENCODING {
            return Err(HttpTransferEncodingNotSupported);
        } else if header.name == header::HOST {
            has_host = true;
        }

        req.headers_mut().expect("Request builder error")
            .append(
                HeaderName::from_bytes(header.name.as_bytes())?,
                HeaderValue::from_bytes(header.value)?
            );
    }

    // Now handle more complex parts of HTTP protocol

    // Handle path according to https://tools.ietf.org/html/rfc2616#section-5.2
    // Tophat ignores the host when determining resource identified. However, the Host header is
    // still required.
    if !has_host {
        return Err(HttpNoHost);
    }
    let path = httparse_req.path.ok_or(HttpNoPath)?;

    // Handling content-length v. transfer-encoding:
    // https://tools.ietf.org/html/rfc7230#section-3.3.3
    let content_length = content_length.unwrap_or(0);

    // Finally build the rest of the req
    let body = reader.take(content_length as u64);
    let req = req
        .method(method)
        .version(version)
        .uri(path)
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
    #[error("Http: no host found")]
    HttpNoHost,
    #[error("Http invalid content length")]
    HttpInvalidContentLength,
    #[error("Http request could not be built")]
    HttpRequestBuild,
    #[error("Http version 1.0 not supported")]
    Http10NotSupported,

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

    // Temporary error until transfer coding supported
    #[error("Http transfer encoding not supported")]
    HttpTransferEncodingNotSupported,
}

pub(crate) fn fail_to_response_and_log(fail: DecodeFail) -> Option<InnerResponse> {
    use log::*;
    use DecodeFail::*;

    // TODO improve logging message
    debug!("Decode error: {} ", fail);

    match fail {
        ConnectionLost(_) => None,
        Http10NotSupported => Some(InnerResponse::version_not_supported()),
        _ => Some(InnerResponse::bad_request()),
    }
}
