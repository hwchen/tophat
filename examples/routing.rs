#![allow(type_alias_bounds)]

// https://users.rust-lang.org/t/how-to-handle-a-vector-of-async-function-pointers/39804
// https://stackoverflow.com/questions/60621816/how-to-indicate-that-the-lifetime-of-an-async-functions-return-value-is-the-sam

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

type Handler<W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static> =
    fn(Request, ResponseWriter<W>, Params) -> Pin<Box<dyn Future<Output = Result<ResponseWritten>> + Send>>;

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
            let router = tree.clone();

            let (stream, _) = listener.accept().await?;
            let stream = Arc::new(stream);

            let task = Task::spawn(async move {
                let serve = accept(stream, |req, resp_wtr| async {
                    let path = "/".to_owned() + req.method().as_str() + req.uri().path();
                    match router.find(&path) {
                        Some((handler, params)) => {
                            // this gets rid of lifetime problems; borrowing the params has issues
                            // with spawning. It's probably fast enough.
                            let params = params.into_iter().map(|(a,b)| (a.to_owned(), b.to_owned())).collect();
                            handler(req, resp_wtr, params).await
                        },
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
async fn hello_user< W>(_req: Request, resp_wtr: ResponseWriter<W>, params: Params) -> Result<ResponseWritten>
    where W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    let mut resp_body = format!("Hello, ");
    for (k, v) in params {
        resp_body.push_str(&format!("{} = {}", k, v));
    }
    let resp = Response::new(resp_body.into());

    resp_wtr.send(resp).await
}

fn pin_hello_user<W>(req: Request, resp_wtr: ResponseWriter<W>, params: Params) -> Pin<Box<dyn Future<Output = Result<ResponseWritten>> + Send>>
    where W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    Box::pin(hello_user(req, resp_wtr, params))
}

//async fn hello_rust<'a, W>(_req: Request, resp_wtr: ResponseWriter<W>, _params: Params<'a>) -> Result<ResponseWritten>
async fn hello_rust<W>(_req: Request, resp_wtr: ResponseWriter<W>, _params: Params) -> Result<ResponseWritten>
    where W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    let resp_body = format!("Hello, rust!");
    let resp = Response::new(resp_body.into());

    resp_wtr.send(resp).await
}

fn pin_hello_rust<W>(req: Request, resp_wtr: ResponseWriter<W>, params: Params) -> Pin<Box<dyn Future<Output = Result<ResponseWritten>> + Send>>
    where W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    Box::pin(hello_rust(req, resp_wtr, params))
}
