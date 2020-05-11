// Thanks to tide, looking at Endpoint helped me understand how to coerce Fn to be storable.
//
// I'm a little nervous, because this router is so much more direct than others I've seen. I think
// it's because I'm not integrating with a Service, and I'm not making middleware.
//
// My main concern was whether there would be such as thing as contention for the function.
// - looks like there's no issue when there's a sleep timer in the fn, the number of connections
// waiting on the sleep still scales with the number of total connections made.
// - Oh, I should test with one autocannon set to a sleep endpoint and another to not.
// - I'm able to run autocannon on `hello_<user>` and simultaneously curl with another user, and it
// goes through just fine. So there's no contention, it's just the sleep.
// - So basically, I think that my code should be fine, the fn's code is just a reference, and
// then runtime-code gets filled in (the appropriate params and stuff).


//! Very Basic router
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

pub struct Router<W>
    where W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    tree: PathTree<Box<dyn Endpoint<W>>>,
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
            Some((endpoint, params)) => {
                let params: Vec<(String, String)> = params.into_iter().map(|(a,b)| (a.to_owned(), b.to_owned())).collect();
                let res = endpoint.call(req, resp_wtr, params).await;
                res
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

    pub fn build(self) -> Arc<Router<W>> {
        Arc::new(Router {
            tree: self.tree,
        })
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
