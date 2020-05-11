use futures_util::io::{AsyncRead, AsyncWrite};
use http::{Method, Response};
use smol::{Async, Task};
use std::net::TcpListener;
use piper::Arc;
use tophat::server::{accept, Params, Request, ResponseWriter, ResponseWritten, Result, Router};

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    let router = Router::new()
        .at(Method::GET, "/:name", hello_user)
        .at(Method::GET, "/", blank)
        .build();

    let listener = Async::<TcpListener>::bind("127.0.0.1:9999")?;

    smol::run(async {
        loop {
            let router = router.clone();

            let (stream, _) = listener.accept().await?;
            let stream = Arc::new(stream);

            let task = Task::spawn(async move {
                let serve = accept(stream, |req, resp_wtr| async {
                    let res = router.route(req, resp_wtr).await;
                    res
                }).await;

                if let Err(err) = serve {
                    eprintln!("Error: {}", err);
                }
            });

            task.detach();
        }
    })
}

async fn hello_user< W>(_req: Request, resp_wtr: ResponseWriter<W>, params: Params) -> Result<ResponseWritten>
    where W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    smol::Timer::after(std::time::Duration::from_secs(5)).await;

    let mut resp_body = format!("Hello, ");
    for (k, v) in params {
        resp_body.push_str(&format!("{} = {}", k, v));
    }
    let resp = Response::new(resp_body.into());

    resp_wtr.send(resp).await
}

async fn blank<W>(_req: Request, resp_wtr: ResponseWriter<W>, _params: Params) -> Result<ResponseWritten>
    where W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    let resp = Response::new(tophat::Body::empty());


    resp_wtr.send(resp).await
}
