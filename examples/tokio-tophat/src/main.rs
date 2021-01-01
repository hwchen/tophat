use async_dup::{Arc, Mutex};
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::net::{self, TcpStream};
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};
use tophat::server::accept;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let listener = net::TcpListener::bind("127.0.0.1:9999").await?;

    loop {
        let (stream, _) = listener.accept().await?;
        let stream = WrapStream::new(stream);

        tokio::spawn(async move {
            let serve = accept(stream, |_req, mut resp_wtr| async {
                let resp_body = "Hello, World!";
                resp_wtr.set_body(resp_body.into());

                resp_wtr.send().await
            })
            .await;

            if let Err(err) = serve {
                eprintln!("Error: {}", err);
            }
        });
    }
}

// TODO I'm not sure this is the best way to do this. Suggestions for simplifying definitely
// welcome. When AsyncRead and AsyncWrite standardized, this shouldn't be necessary.
#[derive(Clone)]
struct WrapStream(Arc<Mutex<Compat<TcpStream>>>);

impl WrapStream {
    fn new(stream: TcpStream) -> Self {
        let stream = stream.compat_write();
        WrapStream(Arc::new(Mutex::new(stream)))
    }
}

impl futures_lite::AsyncRead for WrapStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut *(&*self).0.lock()).poll_read(cx, buf)
    }
}

impl futures_lite::AsyncWrite for WrapStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut *(&*self).0.lock()).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut *(&*self).0.lock()).poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut *(&*self).0.lock()).poll_close(cx)
    }
}
