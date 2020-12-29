use async_dup::Arc;
use smol::Async;
use std::net::TcpListener;
use tophat::server::accept;

fn main() -> Result<(), Box<dyn std::error::Error>> {
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
