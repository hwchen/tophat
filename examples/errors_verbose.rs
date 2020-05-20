use futures_util::io::{AsyncRead, AsyncWrite};
use http::{Method, StatusCode};
use smol::{Async, Task};
use std::net::TcpListener;
use piper::Arc;
use tophat::{
    server::{
        accept_with_opts,
        glitch::{Glitch, GlitchExt, Result},
        router::Router,
        ResponseWriter,
        ResponseWritten,
        ServerOpts,
    },
    Request,
};

const S_500: StatusCode = StatusCode::INTERNAL_SERVER_ERROR;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    let opts = ServerOpts {
        timeout: Some(std::time::Duration::from_secs(60)),
        verbose_glitch: true,
    };

    let router = Router::build()
        .data("Data from datastore")
        .at(Method::GET, "/database_error", database_error)
        .at(Method::GET, "/database_error_context", database_error_context)
        .at(Method::GET, "/missing_data", missing_data)
        .finish();

    let listener = Async::<TcpListener>::bind("127.0.0.1:9999")?;

    smol::run(async {
        loop {
            let router = router.clone();
            let opts = opts.clone();

            let (stream, _) = listener.accept().await?;
            let stream = Arc::new(stream);

            let task = Task::spawn(async move {
                let serve = accept_with_opts(stream, opts, |req, resp_wtr| async {
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

async fn database_error<W>(_req: Request, resp_wtr: ResponseWriter<W>) -> Result<ResponseWritten>
    where W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    use std::io;

    let failed_db: std::result::Result<(), _> = Err(io::Error::new(io::ErrorKind::Other, "The database crashed"));
    failed_db?;

    resp_wtr.send().await
}

async fn database_error_context<W>(_req: Request, resp_wtr: ResponseWriter<W>) -> Result<ResponseWritten>
    where W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    use std::io;

    let failed_db: std::result::Result<(), _> = Err(io::Error::new(io::ErrorKind::Other, "The database crashed"));
    failed_db
        .glitch_ctx(S_500, "looking for user")?;

    resp_wtr.send().await
}

async fn missing_data<W>(_req: Request, resp_wtr: ResponseWriter<W>) -> Result<ResponseWritten>
    where W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    let failed_db = None;

    // Manually create a 400
    // This will work even without anyhow integration.
    failed_db.ok_or_else(|| Glitch::bad_request())?;

    resp_wtr.send().await
}

