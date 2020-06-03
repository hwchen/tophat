// TODO Handle all of the headers. See hyper src/proto/h1/role.rs
// - transfer encoding
// - connection
// - expect
// - upgrade
// etc.

use futures_io::AsyncRead;
use futures_util::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use http::header::{self, HeaderName, HeaderValue};
use std::fmt;

use crate::body::Body;
use crate::chunked::ChunkedDecoder;
use crate::error::Error;
use crate::Request;

use super::response_writer::InnerResponse;

const LF: u8 = b'\n';

const SUPPORTED_TRANSFER_ENCODING: [&[u8]; 2] = [b"chunked", b"identity"];

/// Decode and http request
///
/// Errors are bubbled up and handled in `accept`, the possible decode errors and the error handler
/// are defined in this module.
///
/// `None` means that no request was read.
pub(crate) async fn decode<R>(reader: R) -> Result<Option<Request>, DecodeFail>
where
    R: AsyncRead + Unpin + Send + Sync + 'static,
{
    use DecodeFail::*;

    let mut reader = BufReader::new(reader);
    let mut buf = Vec::new();
    let mut headers = [httparse::EMPTY_HEADER; 16];
    let mut httparse_req = httparse::Request::new(&mut headers);

    // Keep reading bytes from the stream until we hit the end of the head.
    loop {
        let bytes_read = reader
            .read_until(LF, &mut buf)
            .await
            .map_err(ConnectionLost)?;

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
    let status = httparse_req.parse(&buf).map_err(HttpHeadParse)?;
    if status.is_partial() {
        return Err(HttpMalformedHead);
    };

    // Check that req basics are here
    let method = http::Method::from_bytes(httparse_req.method.ok_or(HttpNoMethod)?.as_bytes())
        .map_err(HttpMethod)?;
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
    let mut is_te = false;
    let mut is_chunked = false;
    #[allow(clippy::borrow_interior_mutable_const)] // TODO see if I can remove this later
    for header in httparse_req.headers.iter() {
        if header.name == header::CONTENT_LENGTH {
            content_length = Some(
                std::str::from_utf8(header.value)
                    .map_err(|_| HttpInvalidContentLength)?
                    .parse::<usize>()
                    .map_err(|_| HttpInvalidContentLength)?,
            );
        } else if header.name == header::TRANSFER_ENCODING {
            // return error if transfer encoding not supported
            // TODO this allocates to lowercase ascii. fix?
            if !SUPPORTED_TRANSFER_ENCODING.contains(&header.value.to_ascii_lowercase().as_slice())
            {
                return Err(HttpUnsupportedTransferEncoding);
            }

            is_te = true;
            is_chunked = String::from_utf8_lossy(header.value)
                .trim()
                .eq_ignore_ascii_case("chunked");
        } else if header.name == header::HOST {
            has_host = true;
        }

        req.headers_mut().expect("Request builder error").append(
            HeaderName::from_bytes(header.name.as_bytes()).map_err(HttpHeaderName)?,
            HeaderValue::from_bytes(header.value).map_err(HttpHeaderValue)?,
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
    // TODO double-check with https://tools.ietf.org/html/rfc7230#section-3.3.3
    let content_length = content_length.unwrap_or(0);

    // Decode body as fixed_body or as chunked
    let body = if is_te && is_chunked {
        let mut body = Body::empty();
        let trailer_sender = body.send_trailers();
        let reader = BufReader::new(ChunkedDecoder::new(reader, trailer_sender));
        body.set_inner(reader, None);
        body
    } else {
        Body::from_reader(reader.take(content_length as u64), Some(content_length))
    };

    // Finally build the rest of the req
    let req = req
        .method(method)
        .version(version)
        .uri(path)
        .body(body)
        .map_err(|_| HttpRequestBuild)?;

    Ok(Some(req))
}

// Internal failures. If one leads to an external error to bubble up, convert to a public error in
// the `error` module.
#[derive(Debug)]
pub(crate) enum DecodeFail {
    // These errors should result in a connection closure
    ConnectionLost(std::io::Error),
    HttpMalformedHead,
    HttpUnsupportedTransferEncoding,

    // Below failures should be handled with a Response, but not with connection closure.

    // TODO check that these are actually errors, and not just something to handle
    HttpNoPath,
    HttpNoMethod,
    HttpNoVersion,
    HttpNoHost,
    HttpInvalidContentLength,
    HttpRequestBuild,
    Http10NotSupported,

    // conversions related to http and httparse lib
    HttpHeadParse(httparse::Error),
    HttpMethod(http::method::InvalidMethod),
    HttpHeaderName(http::header::InvalidHeaderName),
    HttpHeaderValue(http::header::InvalidHeaderValue),
}

impl fmt::Display for DecodeFail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use DecodeFail::*;
        match self {
            ConnectionLost(err) => write!(f, "Connection Lost: {}", err),
            HttpMalformedHead => write!(f, "Http parse malformed head"),
            HttpUnsupportedTransferEncoding => write!(f, "Http transfer encoding not supported"),
            HttpNoPath => write!(f, "Http no path found"),
            HttpNoMethod => write!(f, "Http no method found"),
            HttpNoVersion => write!(f, "Http no version found"),
            HttpNoHost => write!(f, "Http no host found"),
            HttpInvalidContentLength => write!(f, "Http invalid content length"),
            HttpRequestBuild => write!(f, "Http request could not be built"),
            Http10NotSupported => write!(f, "Http version 1.0 not supported"),
            HttpHeadParse(err) => write!(f, "Http header parsing error: {}", err),
            HttpMethod(err) => write!(f, "Http Method error: {}", err),
            HttpHeaderName(err) => write!(f, "Http Header name error: {}", err),
            HttpHeaderValue(err) => write!(f, "Http Header value error: {}", err),
        }
    }
}

pub(crate) fn fail_to_response_and_log(fail: &DecodeFail) -> Option<InnerResponse> {
    use log::*;
    use DecodeFail::*;

    // TODO improve logging message
    debug!("Decode error: {} ", fail);

    match fail {
        ConnectionLost(_) => None,
        HttpUnsupportedTransferEncoding => Some(InnerResponse::not_implemented()),
        Http10NotSupported => Some(InnerResponse::version_not_supported()),
        _ => Some(InnerResponse::bad_request()),
    }
}

pub(crate) fn fail_to_crate_err(fail: DecodeFail) -> Option<Error> {
    use log::*;
    use DecodeFail::*;

    // TODO improve logging message
    debug!("Decode crate-level error: {} ", fail);

    match fail {
        //ConnectionLost(err) => Some(Error::ConnectionLost(err)),
        HttpUnsupportedTransferEncoding => Some(Error::ConnectionClosedUnsupportedTransferEncoding),
        _ => None,
    }
}
