#![allow(dead_code)] // because of testing with and without anyhow errors

//! Test Client for testing server
//! Test Server for testing client

use futures_io::{AsyncBufRead, AsyncRead, AsyncWrite};
use std::io;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

#[derive(Clone)]
pub struct Client {
    // bool is true if read/written, fallse if not yet read/written
    // TODO make rdr and wtr structs so this is easier to understand.
    read_buf: Arc<Mutex<(Vec<u8>, bool)>>,
    write_buf: Arc<Mutex<(Vec<u8>, usize)>>,
    expected: Vec<u8>,
    // sometimes writer needs to write more than once, like for chunks
    num_writes: usize,
}

impl Client {
    pub fn new(req: &str, expected_resp: &str) -> Self {
        Self {
            read_buf: Arc::new(Mutex::new((req.to_owned().into_bytes(), false))),
            write_buf: Arc::new(Mutex::new((Vec::new(), 0))),
            expected: expected_resp.to_owned().into_bytes(),
            num_writes: 1,
        }
    }

    pub fn new_with_writes(req: &str, expected_resp: &str, writes: usize) -> Self {
        Self {
            read_buf: Arc::new(Mutex::new((req.to_owned().into_bytes(), false))),
            write_buf: Arc::new(Mutex::new((Vec::new(), 0))),
            expected: expected_resp.to_owned().into_bytes(),
            num_writes: writes,
        }
    }

    pub fn assert(self) {
        let write_buf = self.write_buf.lock().unwrap();
        let resp = remove_date(&write_buf.0);
        assert_eq!(
            String::from_utf8(resp).unwrap(),
            String::from_utf8(self.expected).unwrap()
        );
    }

    pub fn assert_with_resp_date(self, date: &str) {
        let write_buf = self.write_buf.lock().unwrap();

        let resp_with_date = String::from_utf8(write_buf.0.clone()).unwrap();
        resp_with_date.find(date).unwrap();

        let resp = remove_date(&write_buf.0);
        assert_eq!(
            String::from_utf8(resp).unwrap(),
            String::from_utf8(self.expected).unwrap()
        );
    }
}

impl AsyncRead for Client {
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

impl AsyncWrite for Client {
    fn poll_write(self: Pin<&mut Self>, _cx: &mut Context, buf: &[u8]) -> Poll<io::Result<usize>> {
        let mut wtr = self.write_buf.lock().unwrap();
        if wtr.1 < self.num_writes {
            wtr.1 += 1;
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

pub(crate) struct Cursor<T> {
    inner: std::io::Cursor<T>,
}

impl<T> Cursor<T> {
    #[allow(dead_code)]
    pub(crate) fn new(t: T) -> Self {
        Self {
            inner: std::io::Cursor::new(t),
        }
    }
}

impl<T> AsyncRead for Cursor<T>
where
    T: AsRef<[u8]> + Unpin,
{
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        Poll::Ready(std::io::Read::read(&mut self.inner, buf))
    }

    fn poll_read_vectored(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        bufs: &mut [std::io::IoSliceMut<'_>],
    ) -> Poll<std::io::Result<usize>> {
        Poll::Ready(std::io::Read::read_vectored(&mut self.inner, bufs))
    }
}

impl<T> AsyncBufRead for Cursor<T>
where
    T: AsRef<[u8]> + Unpin,
{
    fn poll_fill_buf(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<&'_ [u8]>> {
        Poll::Ready(std::io::BufRead::fill_buf(&mut self.get_mut().inner))
    }

    fn consume(mut self: Pin<&mut Self>, amt: usize) {
        std::io::BufRead::consume(&mut self.inner, amt)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_remove_date() {
        let input =
            b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\ndate: Thu, 07 May 2020 15:54:21 GMT\r\n\r\n";
        let expected = b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\n";

        assert_eq!(
            String::from_utf8(remove_date(input)),
            String::from_utf8(expected.to_vec())
        );
    }
}
