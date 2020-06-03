use async_dup::{Arc, Mutex};
use futures_core::Stream;
use smol::{Async, Task};
use std::net::TcpListener;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use tophat::server::accept;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    let ping_machine = Arc::new(Mutex::new(PingMachine {
        broadcasters: Vec::new(),
    }));

    let listener = Async::<TcpListener>::bind("127.0.0.1:9999")?;

    let ping_task = smol::Task::spawn({
        let ping_machine = ping_machine.clone();
        async move {
            loop {
                ping_machine.lock().ping().await;
                smol::Timer::after(Duration::from_secs(1)).await;
            }
        }
    });
    ping_task.detach();

    smol::run(async {
        loop {
            let (stream, _) = listener.accept().await?;
            let stream = Arc::new(stream);

            let ping_machine = ping_machine.clone();

            let task = Task::spawn(async move {
                let serve = accept(stream, |_req, mut resp_wtr| async {
                    let client = ping_machine.lock().add_client();
                    resp_wtr.set_sse(client);

                    resp_wtr.send().await
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

struct PingMachine {
    broadcasters: Vec<async_channel::Sender<String>>,
}

impl PingMachine {
    async fn ping(&self) {
        for tx in &self.broadcasters {
            let _ = tx.send("data: ping\n\n".to_owned()).await;
        }
    }

    fn add_client(&mut self) -> Client {
        let (tx, rx) = async_channel::bounded(10);

        self.broadcasters.push(tx);

        Client(rx)
    }
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
