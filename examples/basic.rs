use http::header;
use smol::{Async, Task};
use std::net::TcpListener;
use async_dup::Arc;
use tophat::server::accept;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    let listener = Async::<TcpListener>::bind("127.0.0.1:9999")?;

    smol::run(async {
        loop {
            let (stream, _) = listener.accept().await?;
            let stream = Arc::new(stream);

            let task = Task::spawn(async move {
                let serve = accept(stream, |req, mut resp_wtr| async {
                    println!("{:?}", *req.uri());
                    println!("{:?}", req.version());
                    println!("{:?}", req.method());
                    println!("{:?}", req.headers().get(header::CONTENT_LENGTH));
                    println!("{:?}", req.headers().get(header::HOST));

                    let req_body = req.into_body().into_string().await?;
                    let resp_body = format!("Hello, {}!", req_body);
                    resp_wtr.set_body(resp_body.into());

                    resp_wtr.send().await
                }).await;

                if let Err(err) = serve {
                    eprintln!("Error: {}", err);
                }

            });

            task.detach();
        }
    })
}
