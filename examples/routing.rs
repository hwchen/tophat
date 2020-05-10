#![feature(trait_alias)]

use futures_util::io::{AsyncRead, AsyncWrite};
use http::Response;
use smol::{Async, Task};
use std::net::TcpListener;
use path_tree::PathTree;
use std::future::Future;
use piper::Arc;
use tophat::server::{accept, Request, ResponseWriter, ResponseWritten, Result};

pub type Params<'a> = Vec<(&'a str, &'a str)>;
type Handler<W: AsyncWrite + Clone + Send + Sync + Unpin + 'static, Fut: Future<Output = Result<ResponseWritten>>> =
    fn(Request, ResponseWriter<W>) -> Fut;


fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    let mut tree = PathTree::<Handler<_, _>>::new();
    tree.insert("/GET/rust", hello_rust);
    //tree.insert("/GET/:name", hello_user);
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
                            }
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

async fn hello_user<W>(_req: Request, resp_wtr: ResponseWriter<W>) -> Result<ResponseWritten>
    where W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    let mut resp_body = format!("Hello, ");
    //for (k, v) in params {
    //    resp_body.push_str(&format!("{} = {}", k, v));
    //}
    let resp = Response::new(resp_body.into());

    resp_wtr.send(resp).await
}

async fn hello_rust<W>(_req: Request, resp_wtr: ResponseWriter<W>) -> Result<ResponseWritten>
    where W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    let resp_body = format!("Hello, rust!");
    let resp = Response::new(resp_body.into());

    resp_wtr.send(resp).await
}
