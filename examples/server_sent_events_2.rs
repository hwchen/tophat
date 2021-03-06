use async_dup::Arc;
use futures::Stream;
use smol::Async;
use std::net::TcpListener;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use tophat::server::accept;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let listener = Async::<TcpListener>::bind(([127,0,0,1],9999))?;

    smol::block_on(async {
        loop {
            let (stream, _) = listener.accept().await?;
            let stream = Arc::new(stream);

            let task = smol::spawn(async move {
                let serve = accept(stream, |_req, mut resp_wtr| async {
                    let (tx, rx) = async_channel::bounded(100);
                    let client = Client(rx);
                    resp_wtr.set_sse(client);

                    // a one-shot to send the result of the resp_wtr, so that we can exit the
                    // endpoint.
                    let (tx_res, rx_res) = async_channel::bounded(1);

                    smol::spawn(async move {
                        let sse_res = resp_wtr.send().await;
                        let _ = tx_res.send(sse_res).await;
                    })
                    .detach();

                    let _ = tx.send("data: lorem\n\n".to_owned()).await;

                    smol::Timer::after(Duration::from_secs(1)).await;

                    let _ = tx.send("data: ipsum\n\n".to_owned()).await;

                    // This rx will never receive because the stream will never close.
                    //
                    // If the exit from this endpoint was not dependent on the stream closing,
                    // (i.e. `ResponseWritten` could be constructed by user), then the exit of the
                    // endoint would drop the tx client, which would close the stream. However, I
                    // don't think that is idiomatic behavior for an sse, they should be
                    // long-lived.
                    rx_res.recv().await.unwrap()
                })
                .await;

                if let Err(err) = serve {
                    eprintln!("Error: {}", err);
                }
            });

            task.detach();
        }
    })
}

struct Client(async_channel::Receiver<String>);

impl Stream for Client {
    type Item = Result<String, std::io::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.0).poll_next(cx) {
            Poll::Ready(Some(v)) => Poll::Ready(Some(Ok(v))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}
