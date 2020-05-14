use http::Response;
use smol::{Async, Task};
use std::net::TcpListener;
use piper::Arc;
use tophat::{
    server::{
        accept,
        reply,
    },
    some_unwrap_or,
    Body,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    let listener = Async::<TcpListener>::bind("127.0.0.1:9999")?;

    smol::run(async {
        loop {
            let (stream, _) = listener.accept().await?;
            let stream = Arc::new(stream);

            let task = Task::spawn(async move {
                let serve = accept(stream, |_req, resp_wtr| async {
                    let failed_db = None;

                    // not sure about this api... I'll keep it in for now.
                    some_unwrap_or!(
                        failed_db,
                        resp_wtr.send(reply::code(400).unwrap()).await
                    );

                    let resp = Response::new(Body::empty());

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