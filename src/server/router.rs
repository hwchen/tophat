// Thanks to tide, looking at Endpoint helped me understand how to coerce Fn to be storable. And to
// reset-router, to understand how to use extensions and a trait to allow access from a Request.
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
//! - basic routing (no nesting)
//! - holds global data
//! - no extractors (you've got to find all the stuff you want attached to the `Request`)

use crate::server::{Request, ResponseWriter, ResponseWritten, Result};
use async_dup::Arc;
use futures_util::io::{AsyncRead, AsyncWrite};
use http::{Method, StatusCode};
use path_tree::PathTree;
use std::future::Future;
use std::pin::Pin;

/// Convenience type for params.
///
/// A `Vec` of (param_name, captured_value)
pub type Params = Vec<(String, String)>;

/// A minimal router
#[derive(Clone)]
pub struct Router<W>
where
    W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    tree: Arc<PathTree<Box<dyn Endpoint<W>>>>,
    data: Arc<Option<DataMap>>,
}

impl<W> Router<W>
where
    W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    /// Build a router
    pub fn build() -> RouterBuilder<W> {
        RouterBuilder::new()
    }

    /// Call this to route a request
    pub async fn route(
        &self,
        mut req: Request,
        mut resp_wtr: ResponseWriter<W>,
    ) -> Result<ResponseWritten> {
        let path = "/".to_owned() + req.method().as_str() + req.uri().path();

        match self.tree.find(&path) {
            Some((endpoint, params)) => {
                let params: Vec<(String, String)> = params
                    .into_iter()
                    .map(|(a, b)| (a.to_owned(), b.to_owned()))
                    .collect();

                // a place to store data and params
                // extensions is a type map, and then
                // data is also a type map.
                let extensions_mut = req.extensions_mut();
                if let Some(ref data) = *self.data {
                    extensions_mut.insert(data.clone());
                }
                extensions_mut.insert(params);

                endpoint.call(req, resp_wtr).await
            }
            None => {
                resp_wtr.set_status(StatusCode::NOT_FOUND);
                resp_wtr.send().await
            }
        }
    }
}

/// Build a router
pub struct RouterBuilder<W>
where
    W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    tree: PathTree<Box<dyn Endpoint<W>>>,
    data: Option<type_map::concurrent::TypeMap>,
}

impl<W> RouterBuilder<W>
where
    W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    fn new() -> Self {
        Self {
            tree: PathTree::new(),
            data: None,
        }
    }

    /// Attach a route with: method, path, endpoint.
    ///
    /// For the path, you can use:
    /// - Named parameters. e.g. :name.
    /// - Catch-All parameters. e.g. *any, it must always be at the end of the pattern.
    /// - Supports multiple naming for the same path segment. e.g. /users/:id and /users/:user_id/repos.
    /// - Don't care about routes orders, recursive lookup, Static -> Named -> Catch-All.
    /// (path-tree is used as the underlying router)
    pub fn at(self, method: Method, path: &str, endpoint: impl Endpoint<W>) -> Self {
        let mut this = self;

        let path = "/".to_owned() + method.as_str() + path;

        this.tree.insert(&path, Box::new(endpoint));
        this
    }

    /// Add data of type `T` to the router, to be accessed later through the request as
    /// `req.data()`. Data is stored in a typemap.
    ///
    /// Requires `RouterRequestExt`.
    pub fn data<T: Send + Sync + 'static>(self, data: T) -> Self {
        self.wrapped_data(Data::new(data))
    }

    /// Add data of type `Data<T>` to the router, to be accessed later through the request as
    /// `req.data()`. Data is stored in a typemap.
    ///
    /// Requires `RouterRequestExt`.
    pub fn wrapped_data<T: Send + Sync + 'static>(mut self, data: T) -> Self {
        let mut map = self
            .data
            .take()
            .unwrap_or_else(type_map::concurrent::TypeMap::new);
        map.insert(data);
        self.data = Some(map);
        self
    }

    /// Finish building router
    pub fn finish(self) -> Router<W> {
        Router {
            tree: Arc::new(self.tree),
            data: Arc::new(self.data.map(Data::new).map(DataMap)),
        }
    }
}

/// A trait for all endpoints, so that the user can just use any suitable closure or fn in the
/// method for building a router.
pub trait Endpoint<W>: Send + Sync + 'static
where
    W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    /// Invoke the endpoint within the given context
    fn call(
        &self,
        req: Request,
        resp_wtr: ResponseWriter<W>,
    ) -> BoxFuture<Result<ResponseWritten>>;
}

impl<F: Send + Sync + 'static, Fut, Res, W> Endpoint<W> for F
where
    F: Fn(Request, ResponseWriter<W>) -> Fut,
    Fut: Future<Output = Result<Res>> + Send + 'static,
    Res: Into<ResponseWritten>,
    W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    fn call(
        &self,
        req: Request,
        resp: ResponseWriter<W>,
    ) -> BoxFuture<Result<ResponseWritten>> {
        let fut = (self)(req, resp);
        Box::pin(async move {
            let res = fut.await?;
            Ok(res.into())
        })
    }
}

// Router extras: Data and params access

/// Data type for wrapping data for access within an endpoint
pub struct Data<T>(Arc<T>);

impl<T> Data<T> {
    /// Make a Data
    pub fn new(t: T) -> Self {
        Data(Arc::new(t))
    }

    /// Make a Data from data which is wrapped in an Arc
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

/// Trait for convenience methods on a Request, which will allow for retrieving Data and params.
pub trait RouterRequestExt {
    /// Get data
    fn data<T: Send + Sync + 'static>(&self) -> Option<Data<T>>;
    /// Get params
    fn params(&self) -> Option<&Params>;
    /// Get a specific param
    fn get_param(&self, key: &str) -> Option<&str>;
}

impl RouterRequestExt for crate::Request {
    fn data<T: Send + Sync + 'static>(&self) -> Option<Data<T>> {
        self.extensions()
            .get::<DataMap>()
            .and_then(|x| x.0.get::<Data<T>>())
            .cloned()
    }

    fn params(&self) -> Option<&Params> {
        self.extensions().get::<Params>()
    }

    fn get_param(&self, key: &str) -> Option<&str> {
        if let Some(params) = self.extensions().get::<Params>() {
            for (k, v) in params {
                // for right now, just returns first. Is this ok?
                if key == k {
                    return Some(v);
                }
            }
        }
        None
    }
}

pub(crate) type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
