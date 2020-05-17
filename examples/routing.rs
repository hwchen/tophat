use futures_util::io::{AsyncRead, AsyncWrite};
use http::Method;
use smol::{Async, Task};
use std::net::TcpListener;
use piper::Arc;
use tophat::{
    server::{
        accept,
        router::{Router, RouterRequestExt},
        ResponseWriter,
        ResponseWritten,
    },
    Request,
    Result,
};

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    let router = Router::build()
        .data("Data from datastore")
        .at(Method::GET, "/:name", hello_user)
        .at(Method::GET, "/", blank)
        .finish();

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

async fn hello_user< W>(req: Request, mut resp_wtr: ResponseWriter<W>) -> Result<ResponseWritten>
    where W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    //smol::Timer::after(std::time::Duration::from_secs(5)).await;

    let mut resp_body = format!("Hello, ");

    // add params to body string
    if let Some(params) = req.params() {
        for (k, v) in params {
            resp_body.push_str(&format!("{} = {}", k, v));
        }
    }

    // add data to body string
    if let Some(data_string) = req.data::<&str>() {
        resp_body.push_str(&format!(" and {}", *data_string));
    }

    resp_wtr.set_body(resp_body.into());

    resp_wtr.send().await
}

async fn blank<W>(_req: Request, resp_wtr: ResponseWriter<W>) -> Result<ResponseWritten>
    where W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    resp_wtr.send().await
}
