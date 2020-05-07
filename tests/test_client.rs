//! TestClient for testing server

use futures_io::{AsyncRead, AsyncWrite};
use std::io;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

#[derive(Clone)]
pub struct TestClient {
    // bool is true if read/written, fallse if not yet read/written
    // TODO make rdr and wtr structs so this is easier to understand.
    read_buf: Arc<Mutex<(Vec<u8>, bool)>>,
    write_buf: Arc<Mutex<(Vec<u8>, bool)>>,
    expected: Vec<u8>,
}

impl TestClient {
    pub fn new(req: &str, expected_resp: &str) -> Self {
        Self {
            read_buf: Arc::new(Mutex::new((req.to_owned().into_bytes(), false))),
            write_buf: Arc::new(Mutex::new((Vec::new(), false))),
            expected: expected_resp.to_owned().into_bytes(),
        }
    }
    pub fn assert(self) {
        let write_buf = self.write_buf.lock().unwrap();
        let resp = remove_date(&write_buf.0);
        assert_eq!(String::from_utf8(resp).unwrap(), String::from_utf8(self.expected).unwrap());
    }
}

impl AsyncRead for TestClient {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let mut rdr = self.read_buf.lock().unwrap();
        if !rdr.1 {
            rdr.1 = true;
            io::Read::read(&mut io::Cursor::new(&*rdr.0), buf).unwrap();
            Poll::Ready(Ok(rdr.0.len()))
        } else {
            Poll::Ready(Ok(0))
        }
    }
}

impl AsyncWrite for TestClient {
    fn poll_write(self: Pin<&mut Self>, _cx: &mut Context, buf: &[u8]) -> Poll<io::Result<usize>> {
        let mut wtr = self.write_buf.lock().unwrap();
        if !wtr.1 {
            wtr.1 = true;
            wtr.0.extend_from_slice(buf);
            Poll::Ready(Ok(wtr.0.len()))
        } else {
            Poll::Ready(Ok(0))
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(())) // placeholder, shouldn't hit?
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(())) // placeholder, shouldn't hit?
    }
}

// just strip date from response
fn remove_date(b: &[u8]) -> Vec<u8> {
    // just change to str and back is easier for now
    let s = std::str::from_utf8(b).unwrap();
    if let Some(i) = s.find("date: ") {
        let eol = s[i + 6..].find("\r\n").expect("missing date eol");
        let mut res = Vec::new();
        res.extend_from_slice(&b[..i]);
        res.extend_from_slice(&b[i + 6 + eol + 2..]);
        res
    } else {
        b.to_vec()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_remove_date() {
        let input = b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\ndate: Thu, 07 May 2020 15:54:21 GMT\r\n\r\n";
        let expected = b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\n";

        assert_eq!(String::from_utf8(remove_date(input)), String::from_utf8(expected.to_vec()));
    }
}
