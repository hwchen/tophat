#![allow(dead_code)]

use futures_util::io::{AsyncRead, AsyncWrite};
use http::Response;
use smol::{Async, Task};
use std::future::Future;
use std::net::TcpListener;
use std::pin::Pin;
use path_tree::PathTree;
use piper::Arc;
use tophat::server::{accept, Request, ResponseWriter, ResponseWritten, Result};

type Params = Vec<(String, String)>;


fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    //let mut tree = PathTree::<Box<dyn Endpoint<_>>>::new();
    let mut tree = PathTree::<Box<dyn Endpoint<_>>>::new();
    tree.insert("/GET/:name", Box::new(hello_user));
    tree.insert("/GET/rust", Box::new(hello_rust));
    let tree = Arc::new(tree);

    let listener = Async::<TcpListener>::bind("127.0.0.1:9999")?;

    smol::run(async {
        loop {
            let router = Arc::clone(&tree);

            let (stream, _) = listener.accept().await?;
            let stream = Arc::new(stream);

            let task = Task::spawn(async move {
                let serve = accept(stream, |req, resp_wtr| async {

                    let path = "/".to_owned() + req.method().as_str() + req.uri().path();
                    match router.find(&path) {
                        Some((handler, _params)) => handler.call(req, resp_wtr).await,
                        None => {
                            let resp = http::Response::builder()
                                .status(http::StatusCode::NOT_FOUND)
                                .body(tophat::Body::empty())
                                .unwrap();
                            resp_wtr.send(resp).await
                        },
                    }

                }).await;

                if let Err(err) = serve {
                    eprintln!("Error: {}", err);
                }

            });

            task.detach();
        }
    })
}

//async fn hello_user<'a, W>(_req: Request, resp_wtr: ResponseWriter<W>, params: Params<'a>) -> Result<ResponseWritten>
async fn hello_user<W>(_req: Request, resp_wtr: ResponseWriter<W>) -> Result<ResponseWritten>
    where W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    let resp_body = format!("Hello, ");
    //for (k, v) in params {
    //    resp_body.push_str(&format!("{} = {}", k, v));
    //}
    let resp = Response::new(resp_body.into());

    resp_wtr.send(resp).await
}

//async fn hello_rust<'a, W>(_req: Request, resp_wtr: ResponseWriter<W>, _params: Params<'a>) -> Result<ResponseWritten>
async fn hello_rust<W>(_req: Request, resp_wtr: ResponseWriter<W>) -> Result<ResponseWritten>
    where W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    let resp_body = format!("Hello, rust!");
    let resp = Response::new(resp_body.into());

    resp_wtr.send(resp).await
}

pub(crate) type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait Endpoint<W>: Send + Sync + 'static
    where W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    /// Invoke the endpoint within the given context
    fn call<'a>(&'a self, req: Request, resp_wtr: ResponseWriter<W>) -> BoxFuture<'a, Result<ResponseWritten>>;
}

pub(crate) type DynEndpoint<W> = dyn Endpoint<W>;

impl<F: Send + Sync + 'static, Fut, Res, W> Endpoint<W> for F
where
    F: Fn(Request, ResponseWriter<W>) -> Fut,
    Fut: Future<Output = Result<Res>> + Send + 'static,
    Res: Into<ResponseWritten>,
    W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    fn call<'a>(&'a self, req: Request, resp: ResponseWriter<W>) -> BoxFuture<'a, Result<ResponseWritten>> {
        let fut = (self)(req, resp);
        Box::pin(async move {
            let res = fut.await?;
            Ok(res.into())
        })
    }
}
