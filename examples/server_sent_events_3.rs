use futures_core::Stream;
use smol::{Async, Task};
use std::net::TcpListener;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use piper::Arc;
use tophat::server::{accept, reply, ResponseWritten};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    let listener = Async::<TcpListener>::bind("127.0.0.1:9999")?;

    smol::run(async {
        loop {
            let (stream, _) = listener.accept().await?;
            let stream = Arc::new(stream);

            // Unlike server_sent_events_2, this sender is created outside the `accept` scope,
            // so it doesn't get dropped at the end of `accept` and keeps the sse alive.
            let (tx, rx) = piper::chan(100);

            // This is just to show that you can send messages even after the task is spawned (at
            // the end)
            let tx_1 = tx.clone();

            let task = Task::spawn(async move {
                let serve = accept(stream, |_req, resp_wtr| async {
                    let client = Client(rx.clone());
                    let resp = reply::sse(client);

                    smol::Task::spawn(async {
                        let sse = resp_wtr.send(resp).await;

                        println!("hit");

                        if let Err(err) = sse {
                            eprintln!("Error: {}", err);
                        }
                    }).detach();

                    tx.send("data: lorem\n\n".to_owned()).await;

                    smol::Timer::after(Duration::from_secs(1)).await;

                    tx.send("data: ipsum\n\n".to_owned()).await;

                    Ok(ResponseWritten)
                }).await;

                if let Err(err) = serve {
                    eprintln!("Error: {}", err);
                }

            });

            task.detach();

            tx_1.clone().send("You've reached the end".to_owned()).await;
        }
    })
}

struct Client(piper::Receiver<String>);

impl Stream for Client {
    type Item = Result<String, std::io::Error>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.0).poll_next(cx) {
            Poll::Ready(Some(v)) => Poll::Ready(Some(Ok(v))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}


