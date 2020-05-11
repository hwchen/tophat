//! Convenience methods for creating a response.
//!
//! The default is to not set headers, unless the function is used for explicitly creating a
//! content-type.
//!
//! For example, `sse` Server Sent Events, needs a special header set. And `text`
//!
//! All of these Responses can be created manually using `tophat::http::Response` and `tophat::Body`,
//! and then setting on the Response.
//!
//! All of these Responses created by `reply` methods can be altered by using `tophat::http::Response`
//! methods.
//!
//! When creating a Response manually, it's possible for the Body and the Response to be out of
//! sync; for example, it's possible to set the Body using a string, and then set the content-type
//! header on the Response to be `content-type: video/mp4'. The power is in the user's hands.
//!
//! That said, this module aims to makes it easy to create a Response/Body combination that is in sync. There are
//! just no guarantees for whether a valid or desirable combination is sent if altered.
//!
//! All functions in this module should list what headers they modify in the document string, and
//! the type of the parameter should be reflected in the function name (i.e. `text` takes a string,
//! not a stream or reader).
//!
//! Possible body types:
//! - &str/String,
//! - AsyncRead,
//! - Stream (StreamExt),

use futures_util::TryStreamExt;

use crate::Response;
use crate::body::Body;

/// Turn a stream into a Server Sent Events response stream.
/// Adds the content-type header for SSE.
///
/// Takes a `futures::Stream`, and `futures::TryStreamExt` must be in scope.
pub fn sse<S: 'static>(stream: S) -> Response
    where S: TryStreamExt<Error = std::io::Error> + Send + Sync + Unpin,
        S::Ok: AsRef<[u8]> + Send + Sync,
{
    let stream = stream.into_async_read();

    let body = Body::from_reader(stream, None);
    let mut resp = Response::new(body);
    resp.headers_mut().insert(
        "content-type",
        "text/event-stream".parse().unwrap(),
    );

    resp
}

/// Create a response with:
/// - specified code
/// - empty body
/// - no headers
///
/// Returns a `Result` because the code may be invalid.
///
pub fn code(status_code: u16)-> std::result::Result<Response, http::status::InvalidStatusCode> {
    let mut resp = Response::new(Body::empty());
    *resp.status_mut() = http::StatusCode::from_u16(status_code)?;

    Ok(resp)
}

/// Create a response with:
/// - specified code
/// - no headers
/// - Body
///
/// The Body can be from AsyncRead or a buffer, the user must manually create the Body.
/// If Server Sent Events are desired, and a stream is converted into AsyncRead and fed
/// into the Body, no content-type header will be set. (Using the `sse` function would set the
/// content-type header).
///
/// Returns a `Result` because the code may be invalid.
///
pub fn code_with_body(status_code: u16, body: Body)-> std::result::Result<Response, http::status::InvalidStatusCode> {
    let mut resp = Response::new(body);
    *resp.status_mut() = http::StatusCode::from_u16(status_code)?;

    Ok(resp)
}

/// Create a response with:
/// - 200 OK
/// - Content-type
/// - Body from String
///
pub fn text(s: String)-> Response {
    let mut resp = Response::new(s.into());
    resp.headers_mut().append(http::header::CONTENT_TYPE, "text/plain".parse().unwrap());

    resp
}

#[macro_export]
macro_rules! some_unwrap_or {
    ($option_expr: expr, $http_resp:expr) => {
        match $option_expr {
            Some(x) => x,
            None => return $http_resp,
        }
    };
}

#[macro_export]
macro_rules! ok_unwrap_or{
    ($option_expr: expr, $http_resp:expr) => {
        match $option_expr {
            Ok(x) => x,
            Err(err) => {
                return $http_resp;
            }
        }
    };
}
