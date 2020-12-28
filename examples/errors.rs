use async_dup::Arc;
use futures_util::io::{AsyncRead, AsyncWrite};
use http::Method;
use smol::{Async, Task};
use std::net::TcpListener;
use tophat::{
    server::{
        accept,
        glitch::{Glitch, Result},
        router::Router,
        ResponseWriter, ResponseWritten,
    },
    Request,
};

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let router = Router::build()
        .data("Data from datastore")
        .at(Method::GET, "/database_error", database_error)
        .at(Method::GET, "/missing_data", missing_data)
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

async fn database_error<W>(_req: Request, resp_wtr: ResponseWriter<W>) -> Result<ResponseWritten>
where
    W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    use std::io;

    let failed_db = Err(io::Error::new(io::ErrorKind::Other, ""));
    failed_db?; // returns a 500 automatically.

    resp_wtr.send().await
}

async fn missing_data<W>(_req: Request, resp_wtr: ResponseWriter<W>) -> Result<ResponseWritten>
where
    W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    let failed_db = None;

    // Manually create a 400
    // This will work even without anyhow integration.
    failed_db.ok_or_else(|| Glitch::bad_request())?;

    resp_wtr.send().await
}
