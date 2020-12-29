use async_channel::unbounded;
use async_dup::Arc;
use easy_parallel::Parallel;
use smol::{future, Async, Executor};
use std::net::TcpListener;
use tophat::server::accept;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ex = Executor::new();
    let (signal, shutdown) = unbounded::<()>();

    Parallel::new()
        .each(0..num_cpus::get().max(1), |_| future::block_on(ex.run(shutdown.recv())))
        .finish(|| future::block_on(async {
            drop(signal);
        }));

    let listener = Async::<TcpListener>::bind(([127,0,0,1],9999))?;

    smol::block_on(async {
        loop {
            let (stream, _) = listener.accept().await?;
            let stream = Arc::new(stream);

            let task = smol::spawn(async move {
                let serve = accept(stream, |_req, resp_wtr| async { resp_wtr.send().await }).await;

                if let Err(err) = serve {
                    eprintln!("Error: {}", err);
                }
            });

            task.detach();
        }
    })
}
