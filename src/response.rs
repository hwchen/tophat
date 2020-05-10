use http::Response as HttpResponse;

use crate::body::Body;

/// Currently, Response is not generic over Body type
pub type Response = HttpResponse<Body>;
