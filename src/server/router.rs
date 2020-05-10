// Thanks to tide, looking at Endpoint helped me understand how to coerce Fn to be storable.

//! Basic router
//!

use futures_util::io::{AsyncRead, AsyncWrite};
use http::Method;
use std::future::Future;
use std::pin::Pin;
use path_tree::PathTree;
use piper::Arc;
use crate::Body;
use crate::server::{Request, ResponseWriter, ResponseWritten, Result};

pub type Params = Vec<(String, String)>;

#[derive(Clone)]
pub struct Router<W>
    where W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    tree: Arc<PathTree<Box<dyn Endpoint<W>>>>,
}

impl<W> Router<W>
    where W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    pub fn new() -> RouterBuilder<W> {
        RouterBuilder::new()
    }

    pub async fn route(&self, req: Request, resp_wtr: ResponseWriter<W>) -> Result<ResponseWritten> {
        let path = "/".to_owned() + req.method().as_str() + req.uri().path();

        match self.tree.find(&path) {
            Some((handler, params)) => {
                let params = params.into_iter().map(|(a,b)| (a.to_owned(), b.to_owned())).collect();
                handler.call(req, resp_wtr, params).await
            },
            None => {
                let resp = http::Response::builder()
                    .status(http::StatusCode::NOT_FOUND)
                    .body(Body::empty())
                    .unwrap();
                resp_wtr.send(resp).await
            },
        }
    }
}

pub struct RouterBuilder<W>
    where W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    tree: PathTree<Box<dyn Endpoint<W>>>,
}

impl<W> RouterBuilder<W>
    where W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    pub fn new() -> Self {
        Self {
            tree: PathTree::new(),
        }
    }

    pub fn at(&mut self, method: Method, path: &str, endpoint: impl Endpoint<W>) -> &mut Self {
        let path = "/".to_owned() + method.as_str() + path;

        self.tree.insert(&path, Box::new(endpoint));
        self
    }

    pub fn build(self) -> Router<W> {
        Router {
            tree: Arc::new(self.tree),
        }
    }
}

pub trait Endpoint<W>: Send + Sync + 'static
    where W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    /// Invoke the endpoint within the given context
    fn call<'a>(&'a self, req: Request, resp_wtr: ResponseWriter<W>, params: Params) -> BoxFuture<'a, Result<ResponseWritten>>;
}

impl<F: Send + Sync + 'static, Fut, Res, W> Endpoint<W> for F
where
    F: Fn(Request, ResponseWriter<W>, Params) -> Fut,
    Fut: Future<Output = Result<Res>> + Send + 'static,
    Res: Into<ResponseWritten>,
    W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    fn call<'a>(&'a self, req: Request, resp: ResponseWriter<W>, params: Params) -> BoxFuture<'a, Result<ResponseWritten>> {
        let fut = (self)(req, resp, params);
        Box::pin(async move {
            let res = fut.await?;
            Ok(res.into())
        })
    }
}

pub(crate) type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
