#![allow(clippy::nonminimal_bool)]
#![allow(clippy::op_ref)]

use futures_io::AsyncRead;
use futures_util::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use http::{
    header::{HeaderMap, HeaderName, HeaderValue, CONTENT_LENGTH, DATE, TRANSFER_ENCODING},
    StatusCode,
};
use httpdate::fmt_http_date;

use super::error::{self, ClientError};
use crate::chunked::ChunkedDecoder;
use crate::{Body, Response};

const CR: u8 = b'\r';
const LF: u8 = b'\n';
const MAX_HEADERS: usize = 128;
const MAX_HEAD_LENGTH: usize = 8 * 1024;

/// Decode an HTTP response on the client.
#[doc(hidden)]
pub async fn decode<R>(reader: R) -> Result<Response, ClientError>
where
    R: AsyncRead + Unpin + Send + Sync + 'static,
{
    let mut reader = BufReader::new(reader);
    let mut buf = Vec::new();
    let mut headers = [httparse::EMPTY_HEADER; MAX_HEADERS];
    let mut httparse_res = httparse::Response::new(&mut headers);

    // Keep reading bytes from the stream until we hit the end of the stream.
    loop {
        let bytes_read = reader
            .read_until(LF, &mut buf)
            .await
            .map_err(error::decode_err)?;
        // No more bytes are yielded from the stream.
        if !(bytes_read != 0) {
            error::decode("Empty response".to_owned());
        }

        // Prevent CWE-400 DDOS with large HTTP Headers.
        if !(buf.len() < MAX_HEAD_LENGTH) {
            return Err(error::decode(
                "Head byte length should be less than 8kb".to_owned(),
            ));
        };

        // We've hit the end delimiter of the stream.
        let idx = buf.len() - 1;
        if idx >= 3 && &buf[idx - 3..=idx] == [CR, LF, CR, LF] {
            break;
        }
        if idx >= 1 && &buf[idx - 1..=idx] == [LF, LF] {
            break;
        }
    }

    // Convert our header buf into an httparse instance, and validate.
    let status = httparse_res.parse(&buf).map_err(error::decode_err)?;
    if status.is_partial() {
        return Err(error::decode("Malformed HTTP head".to_owned()));
    };

    let code = httparse_res.code;
    let code = code.ok_or_else(|| error::decode("No status code found".to_owned()))?;

    // Convert httparse headers + body into a `http_types::Response` type.
    let version = httparse_res.version;
    let version = version.ok_or_else(|| error::decode("No version found".to_owned()))?;
    if version != 1 {
        return Err(error::decode("Unsupported HTTP version".to_owned()));
    };

    let mut headers = HeaderMap::new();
    for header in httparse_res.headers.iter() {
        let value = HeaderValue::from_bytes(header.value).map_err(error::decode_err)?;
        let name: HeaderName = header.name.parse().map_err(error::decode_err)?;
        headers.append(name, value);
    }

    if headers.get(DATE).is_none() {
        let date = fmt_http_date(std::time::SystemTime::now());
        let value = HeaderValue::from_str(&date).map_err(error::decode_err)?;
        headers.insert(DATE, value);
    }

    let content_length = headers.get(CONTENT_LENGTH);
    let transfer_encoding = headers.get(TRANSFER_ENCODING);

    if !(content_length.is_none() || transfer_encoding.is_none()) {
        return Err(error::decode("Unexpected Content-Length header".to_owned()));
    };

    // must be either transfer encoding or content length/ TODO compile time
    let mut res = Response::new(Body::empty());

    if let Some(encoding) = headers.get(TRANSFER_ENCODING).iter().last() {
        if *encoding == "chunked" {
            let mut body = Body::empty();
            let trailers_sender = body.send_trailers();
            let reader = BufReader::new(ChunkedDecoder::new(reader, trailers_sender));
            body.set_inner(reader, None);
            *res.body_mut() = body;

            // Return the response.
            return Ok(res);
        }
    }

    // Check for Content-Length.
    if let Some(len) = headers.get(CONTENT_LENGTH).iter().last() {
        let len = len
            .to_str()
            .map_err(error::decode_err)?
            .parse::<usize>()
            .map_err(error::decode_err)?;
        res = Response::new(Body::from_reader(reader.take(len as u64), Some(len)));
    }

    *res.status_mut() = StatusCode::from_u16(code).map_err(error::decode_err)?;

    *res.headers_mut() = headers;

    // Return the response.
    Ok(res)
}
