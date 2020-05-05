use futures_io::AsyncRead;
use futures_util::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use http::{Request as HttpRequest, header::CONTENT_LENGTH};

use crate::body::Body;
use crate::Request;

const LF: u8 = b'\n';

pub(crate) async fn decode<R>(addr: &str, reader: R) -> http::Result<Option<Request>>
where
    R: AsyncRead + Unpin + Send + Sync + 'static
{
    let mut reader = BufReader::new(reader);
    let mut buf = Vec::new();
    let mut headers = [httparse::EMPTY_HEADER; 16];
    let mut httparse_req = httparse::Request::new(&mut headers);

    // Keep reading bytes from the stream until we hit the end of the stream.
    loop {
        let bytes_read = reader.read_until(LF, &mut buf).await.unwrap();
        println!("buf");
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
    let status = httparse_req.parse(&buf).unwrap();

    // just check for content length for now
    let mut content_length = None;
    for header in httparse_req.headers.iter() {
        println!("header name: {}", header.name);
        if header.name == CONTENT_LENGTH {
            content_length = Some(std::str::from_utf8(header.value).unwrap().parse::<usize>().unwrap());
        }
    }

    let content_length = content_length.unwrap();
    println!("content-length: {}", content_length);

    let body = reader.take(content_length as u64);
    let req = HttpRequest::new(Body::from_reader(body, Some(content_length)));

    Ok(Some(req))
}
