use http::Request as HttpRequest;

use crate::body::Body;

/// Currently, Request is not generic over Body type
pub type Request = HttpRequest<Body>;

