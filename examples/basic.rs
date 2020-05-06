use http::Response;
use smol::{Async, Task};
use std::net::TcpListener;
use piper::Arc;
use tophat::accept;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = Async::<TcpListener>::bind("127.0.0.1:9999")?;
    let addr = format!("http://{}", listener.get_ref().local_addr()?);

    smol::run(async {
        loop {
            let addr = addr.clone();
            let (stream, _) = listener.accept().await?;
            let stream = Arc::new(stream);

            let task = Task::spawn(async move {
                let serve = accept(&addr, stream, |req, resp_wtr| async {
                    let req_body = req.into_body().into_string().await.unwrap();
                    let resp_body = format!("Hello, {}!", req_body);
                    let resp = Response::new(resp_body.into());

                    resp_wtr.send(resp).await
                }).await;

                if let Err(err) = serve {
                    eprintln!("Error: {}", err);
                }

            });

            task.detach();
        }
    })
}
