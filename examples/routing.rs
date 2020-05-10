#![allow(dead_code)]
#![allow(type_alias_bounds)]

use futures_util::io::{AsyncRead, AsyncWrite};
use http::Response;
use smol::{Async, Task};
use std::future::Future;
use std::net::TcpListener;
use std::pin::Pin;
use path_tree::PathTree;
use piper::Arc;
use tophat::server::{accept, Request, ResponseWriter, ResponseWritten, Result};

type Params<'a> = Vec<(&'a str, &'a str)>;

type Handler<W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static> = fn(Request, ResponseWriter<W>) -> Pin<Box<dyn Future<Output = Result<ResponseWritten>> + Send>>;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    //let mut tree = PathTree::<Box<dyn Endpoint<_>>>::new();
    let mut tree = PathTree::<Handler<_>>::new();
    tree.insert("/GET/:name", pin_hello_user);
    tree.insert("/GET/rust", pin_hello_rust);
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
                        Some((handler, _params)) => handler(req, resp_wtr).await,
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

fn pin_hello_user<W>(req: Request, resp_wtr: ResponseWriter<W>) -> Pin<Box<dyn Future<Output = Result<ResponseWritten>>+ Send>>
    where W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    Box::pin(hello_user(req, resp_wtr))
}

//async fn hello_rust<'a, W>(_req: Request, resp_wtr: ResponseWriter<W>, _params: Params<'a>) -> Result<ResponseWritten>
async fn hello_rust<W>(_req: Request, resp_wtr: ResponseWriter<W>) -> Result<ResponseWritten>
    where W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    let resp_body = format!("Hello, rust!");
    let resp = Response::new(resp_body.into());

    resp_wtr.send(resp).await
}

fn pin_hello_rust<W>(req: Request, resp_wtr: ResponseWriter<W>) -> Pin<Box<dyn Future<Output = Result<ResponseWritten>>+ Send>>
    where W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    Box::pin(hello_rust(req, resp_wtr))
}
