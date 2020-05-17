// Cors module based on warp's.

//! Cors module
//!
//! Handles pre-flight
//!
//! Currently a super-simple, not-complete implementation.
//!
//! Does _not_ check for correctness of request headers and content-type.
// (Does anybody? I checked warp and iron cors middleware, I don't think they do.
//!
//! Not yet an ergonomic api. (No builder)
//!
//! ## Simple cors
//! Only checks for client's Origin header, and will respond with a `Access-Control-Allow-Origin`
//! header only, with the specified allowed origins.
//!
//! ## Preflight cors
//! - client method: is `Options`
//! - client header: origin
//! - client header: access-control-request-method
//! - client header: access-control-request-headers
//!
//! - server status: 200 OK
//! - server header: access-control-allow-origin
//! - server header: access-control-allow-methods
//! - server header: access-control-allow-headers
//! - server header: access-control-max-age (86400s is one day)

use futures_util::io::{AsyncRead, AsyncWrite};
use headers::{AccessControlAllowHeaders, AccessControlAllowMethods, AccessControlExposeHeaders, HeaderMapExt, Origin};
use http::{header::{self, HeaderName, HeaderValue}, Method};
use std::collections::HashSet;
use std::convert::TryFrom;

use crate::{server::ResponseWriter, Request};

pub struct CorsBuilder {
    /// For preflight and simple, whether to add the access-control-allow-credentials header
    /// default false
    pub credentials: bool,
    /// For preflight only, allowed headers
    pub allowed_headers: HashSet<HeaderName>,
    /// For preflight and simple, tell client what headers it can access
    pub exposed_headers: HashSet<HeaderName>,
    /// For preflight only, max age
    pub max_age: Option<u64>,
    /// For preflight only, allowed methods
    pub methods: HashSet<http::Method>,
    /// For preflight and simple, allowed origins. Default is '*'
    pub origins: Option<HashSet<HeaderValue>>,
}

impl CorsBuilder {
    /// Sets whether to add the `Access-Control-Allow-Credentials` header.
    pub fn allow_credentials(mut self, allow: bool) -> Self {
        self.credentials = allow;
        self
    }

    /// Adds a method to the existing list of allowed request methods.
    ///
    /// # Panics
    ///
    /// Panics if the provided argument is not a valid `http::Method`.
    pub fn allow_method<M>(mut self, method: M) -> Self
    where
        http::Method: TryFrom<M>,
    {
        let method = match TryFrom::try_from(method) {
            Ok(m) => m,
            Err(_) => panic!("illegal Method"),
        };
        self.methods.insert(method);
        self
    }

    /// Adds multiple methods to the existing list of allowed request methods.
    ///
    /// # Panics
    ///
    /// Panics if the provided argument is not a valid `http::Method`.
    pub fn allow_methods<I>(mut self, methods: I) -> Self
    where
        I: IntoIterator,
        http::Method: TryFrom<I::Item>,
    {
        let iter = methods.into_iter().map(|m| match TryFrom::try_from(m) {
            Ok(m) => m,
            Err(_) => panic!("illegal Method"),
        });
        self.methods.extend(iter);
        self
    }

    /// Adds a header to the list of allowed request headers.
    ///
    /// # Panics
    ///
    /// Panics if the provided argument is not a valid `http::header::HeaderName`.
    pub fn allow_header<H>(mut self, header: H) -> Self
    where
        HeaderName: TryFrom<H>,
    {
        let header = match TryFrom::try_from(header) {
            Ok(m) => m,
            Err(_) => panic!("illegal Header"),
        };
        self.allowed_headers.insert(header);
        self
    }

    /// Adds multiple headers to the list of allowed request headers.
    ///
    /// # Panics
    ///
    /// Panics if any of the headers are not a valid `http::header::HeaderName`.
    pub fn allow_headers<I>(mut self, headers: I) -> Self
    where
        I: IntoIterator,
        HeaderName: TryFrom<I::Item>,
    {
        let iter = headers.into_iter().map(|h| match TryFrom::try_from(h) {
            Ok(h) => h,
            Err(_) => panic!("illegal Header"),
        });
        self.allowed_headers.extend(iter);
        self
    }

    /// Adds a header to the list of exposed headers.
    ///
    /// # Panics
    ///
    /// Panics if the provided argument is not a valid `http::header::HeaderName`.
    pub fn expose_header<H>(mut self, header: H) -> Self
    where
        HeaderName: TryFrom<H>,
    {
        let header = match TryFrom::try_from(header) {
            Ok(m) => m,
            Err(_) => panic!("illegal Header"),
        };
        self.exposed_headers.insert(header);
        self
    }

    /// Adds multiple headers to the list of exposed headers.
    ///
    /// # Panics
    ///
    /// Panics if any of the headers are not a valid `http::header::HeaderName`.
    pub fn expose_headers<I>(mut self, headers: I) -> Self
    where
        I: IntoIterator,
        HeaderName: TryFrom<I::Item>,
    {
        let iter = headers.into_iter().map(|h| match TryFrom::try_from(h) {
            Ok(h) => h,
            Err(_) => panic!("illegal Header"),
        });
        self.exposed_headers.extend(iter);
        self
    }

    /// Sets that *any* `Origin` header is allowed.
    ///
    /// # Warning
    ///
    /// This can allow websites you didn't instead to access this resource,
    /// it is usually better to set an explicit list.
    pub fn allow_any_origin(mut self) -> Self {
        self.origins = None;
        self
    }

    /// Add an origin to the existing list of allowed `Origin`s.
    ///
    /// # Panics
    ///
    /// Panics if the provided argument is not a valid `Origin`.
    pub fn allow_origin(self, origin: impl IntoOrigin) -> Self {
        self.allow_origins(Some(origin))
    }

    /// Add multiple origins to the existing list of allowed `Origin`s.
    ///
    /// # Panics
    ///
    /// Panics if the provided argument is not a valid `Origin`.
    pub fn allow_origins<I>(mut self, origins: I) -> Self
    where
        I: IntoIterator,
        I::Item: IntoOrigin,
    {
        let iter = origins
            .into_iter()
            .map(IntoOrigin::into_origin)
            .map(|origin| {
                origin
                    .to_string()
                    .parse()
                    .expect("Origin is always a valid HeaderValue")
            });

        self.origins.get_or_insert_with(HashSet::new).extend(iter);

        self
    }

    /// Sets the `Access-Control-Max-Age` header.
    pub fn max_age(mut self, seconds: u64) -> Self {
        self.max_age = Some(seconds);
        self
    }

    pub fn finish(self) -> Cors {
        let exposed_headers = if self.exposed_headers.is_empty() {
            None
        } else {
            Some(self.exposed_headers.into_iter().collect())
        };

        Cors {
            credentials: self.credentials,
            allowed_headers: self.allowed_headers.iter().cloned().collect(),
            allowed_headers_set: self.allowed_headers,
            exposed_headers,
            max_age: self.max_age,
            methods: self.methods.iter().cloned().collect(),
            methods_set: self.methods,
            origins: self.origins,
        }
    }
}

#[derive(Clone)]
pub struct Cors {
    /// For preflight and simple, whether to add the access-control-allow-credentials header
    /// default false
    credentials: bool,

    allowed_headers_set: HashSet<HeaderName>,
    /// For preflight only, allowed headers
    allowed_headers: AccessControlAllowHeaders,

    /// For preflight and simple, tell client what headers it can access
    exposed_headers: Option<AccessControlExposeHeaders>,

    /// For preflight only, max age
    max_age: Option<u64>,

    methods_set: HashSet<http::Method>,
    /// For preflight only, allowed methods
    methods: AccessControlAllowMethods,
    /// For preflight and simple, allowed origins. Default is '*'
    /// When responding, just use the origin sent by client if it's in the allowed list.
    origins: Option<HashSet<HeaderValue>>,
}

impl Cors {
    pub fn new() -> CorsBuilder {
        CorsBuilder {
            credentials: false,
            allowed_headers:HashSet::new(),
            exposed_headers:HashSet::new(),
            max_age: None,
            methods: HashSet::new(),
            origins: None,
        }
    }

    // `Options` method differentiates preflight from simple. Does not check for correctness of a
    // simple request.
    pub fn validate<W>(&self, req: &Request, resp_wtr: &mut ResponseWriter<W>) -> Validated
        where W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
    {
        let req_method = req.method();
        let req_origin = req.headers().get(header::ORIGIN);

        match (req_method, req_origin) {
            (&Method::OPTIONS, Some(origin)) => {
                // Preflight checks
                if !self.is_origin_allowed(origin) {
                    return Validated::Invalid;
                    // TODO
                    //Err(Forbidden::OriginNotAllowed);
                }

                let headers = resp_wtr.response_mut().headers_mut();

                if let Some(req_method) = headers.get(header::ACCESS_CONTROL_REQUEST_METHOD) {
                    if !self.is_method_allowed(req_method) {
                        return Validated::Invalid;
                        // TODO error handling
                        //Err(Forbidden::MethodNotAllowed);
                    }
                } else {
                    return Validated::Invalid;
                    // TODO error handling
                    // return Err(Forbidden::MethodNotAllowed);
                }

                if let Some(req_headers) = headers.get(header::ACCESS_CONTROL_REQUEST_HEADERS) {
                    // TODO error handling
                    //let headers = req.headers()
                    //    .to_str()
                    //    .map_err(|_| Forbidden::HeaderNotAllowed)?;
                    let headers = match req_headers.to_str() {
                        Ok(h) => h,
                        Err(_) => return Validated::Invalid,
                    };
                    for header in headers.split(',') {
                        if !self.is_header_allowed(header) {
                            return Validated::Invalid;
                            // TODO error handling
                            //return Err(Forbidden::HeaderNotAllowed);
                        }
                    }
                }

                // If all checks successful, continue with headers for resp.
                // set headers
                self.append_preflight_headers(resp_wtr);
                // set allowed-origin header
                resp_wtr.insert_header(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin.clone());

                Validated::Preflight
            },
            (_, Some(origin)) => {
                // Simple
                if self.is_origin_allowed(origin) {
                    // set common headers
                    self.append_common_headers(resp_wtr);
                    // set allowed-origin header
                    resp_wtr.insert_header(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin.clone());

                    return Validated::Simple;
                }

                // If origin is not allowed
                Validated::Invalid
            },
            (_, _) => {
                // All other requests are not Cors
                Validated::NotCors
            },
        }
    }

    fn is_method_allowed(&self, header: &HeaderValue) -> bool {
        http::Method::from_bytes(header.as_bytes())
            .map(|method| self.methods_set.contains(&method))
            .unwrap_or(false)
    }

    fn is_header_allowed(&self, header: &str) -> bool {
        HeaderName::from_bytes(header.as_bytes())
            .map(|header| self.allowed_headers_set.contains(&header))
            .unwrap_or(false)
    }

    fn is_origin_allowed(&self, origin: &HeaderValue) -> bool {
        if let Some(ref allowed) = self.origins {
            allowed.contains(origin)
        } else {
            true
        }
    }

    fn append_preflight_headers<W>(&self, resp_wtr: &mut ResponseWriter<W>)
        where W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
    {
        self.append_common_headers(resp_wtr);

        let headers = resp_wtr.response_mut().headers_mut();

        headers.typed_insert(self.allowed_headers.clone());
        headers.typed_insert(self.methods.clone());

        if let Some(max_age) = self.max_age {
            headers.insert(header::ACCESS_CONTROL_MAX_AGE, max_age.into());
        }
    }

    fn append_common_headers<W>(&self, resp_wtr: &mut ResponseWriter<W>)
        where W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
    {
        let headers = resp_wtr.response_mut().headers_mut();

        if self.credentials {
            headers.insert(
                header::ACCESS_CONTROL_ALLOW_CREDENTIALS,
                HeaderValue::from_static("true"),
            );
        }
        if let Some(expose_headers_header) = &self.exposed_headers {
            headers.typed_insert(expose_headers_header.clone())
        }
    }
}

pub enum Validated {
    // proceed to endpoint
    Simple,
    // proceed to endpoint
    NotCors,
    // early return
    Preflight,
    // early return
    Invalid,
}

pub trait IntoOrigin {
    fn into_origin(self) -> Origin;
}

impl<'a> IntoOrigin for &'a str {
    fn into_origin(self) -> Origin {
        let mut parts = self.splitn(2, "://");
        let scheme = parts.next().expect("missing scheme");
        let rest = parts.next().expect("missing scheme");

        Origin::try_from_parts(scheme, rest, None).expect("invalid Origin")
    }
}
