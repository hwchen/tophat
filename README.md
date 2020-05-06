# tophat

An async HTTP server. Currently in pre-alpha.

This:
```rust
async fn handler(req:Request, resp_wtr: ResponseWriter) -> Result<ResponseWritten, Error> {
    let done = resp_wtr.send(Response::empty())?;
    // Do things here after resp is written
    Ok(done)
}
```

instead of:
```rust
async fn handler(req:Request) -> Result<Response, Error> {
    Ok(Response::empty())
}
```

# Features
- Works with any tcp stream that implements `futures::{AsyncRead, AsyncWrite}`.
- All dependencies are async-ecosystem independent.
- Fast enough.

# Upcoming
- Transfer-encoding
- Completely correct handling of HTTP protocol
- Request/Responpse logging
