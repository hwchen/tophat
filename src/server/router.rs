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

#[derive(Clone)]
pub struct Router<W>
    where W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    tree: Arc<PathTree<Box<dyn Endpoint<W>>>>,
    data: Arc<Option<DataMap>>,
}

impl<W> Router<W>
    where W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    pub fn new() -> RouterBuilder<W> {
        RouterBuilder::new()
    }

    pub async fn route(&self, mut req: Request, resp_wtr: ResponseWriter<W>) -> Result<ResponseWritten> {
        let path = "/".to_owned() + req.method().as_str() + req.uri().path();

        match self.tree.find(&path) {
            Some((endpoint, params)) => {
                let params: Vec<(String, String)> = params.into_iter().map(|(a,b)| (a.to_owned(), b.to_owned())).collect();

                // a place to store data and params
                // extensions is a type map, and then
                // data is also a type map.
                let extensions_mut = req.extensions_mut();
                if let Some(ref data) = *self.data {
                    extensions_mut.insert(data.clone());
                }
                extensions_mut.insert(params);

                let res = endpoint.call(req, resp_wtr).await;
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
    data: Option<type_map::concurrent::TypeMap>,
}

impl<W> RouterBuilder<W>
    where W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    pub fn new() -> Self {
        Self {
            tree: PathTree::new(),
            data: None,
        }
    }

    pub fn at(self, method: Method, path: &str, endpoint: impl Endpoint<W>) -> Self {
        let mut this = self;

        let path = "/".to_owned() + method.as_str() + path;

        this.tree.insert(&path, Box::new(endpoint));
        this
    }

    pub fn data<T: Send + Sync + 'static>(self, data: T) -> Self {
        self.wrapped_data(Data::new(data))
    }

    pub fn wrapped_data<T: Send + Sync + 'static>(mut self, data: T) -> Self {
        let mut map = self.data.take().unwrap_or_else(type_map::concurrent::TypeMap::new);
        map.insert(data);
        self.data = Some(map);
        self
    }

    pub fn build(self) -> Router<W> {
        Router {
            tree: Arc::new(self.tree),
            data: Arc::new(self.data.map(Data::new).map(DataMap)),
        }
    }
}

pub trait Endpoint<W>: Send + Sync + 'static
    where W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    /// Invoke the endpoint within the given context
    fn call<'a>(&'a self, req: Request, resp_wtr: ResponseWriter<W>) -> BoxFuture<'a, Result<ResponseWritten>>;
}

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

// Router extras: Data and params access

pub struct Data<T>(Arc<T>);

impl<T> Data<T> {
    pub fn new(t: T) -> Self {
        Data(Arc::new(t))
    }

    pub fn from_arc(arc: Arc<T>) -> Self {
        Data(arc)
    }
}

impl<T> std::ops::Deref for Data<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl<T> Clone for Data<T> {
    fn clone(&self) -> Self {
        Data(Arc::clone(&self.0))
    }
}

#[derive(Clone)]
struct DataMap(Data<type_map::concurrent::TypeMap>);

pub trait RouterRequestExt {
    fn data<T: Send + Sync + 'static>(&self) -> Option<Data<T>>;
    fn params(&self) -> Option<&Params>;
}

impl RouterRequestExt for crate::Request {
    fn data<T: Send + Sync + 'static>(&self) -> Option<Data<T>> {
        self.extensions().get::<DataMap>().and_then(|x| x.0.get::<Data<T>>()).cloned()
    }

    fn params(&self) -> Option<&Params> {
        self.extensions().get::<Params>()
    }
}

pub(crate) type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
