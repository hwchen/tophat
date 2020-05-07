# tophat
[![CI](https://github.com/hwchen/tophat/workflows/ci/badge.svg)](https://github.com/hwchen/tophat/actions?query=workflow%3Aci)

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
- HTTP/1
- Works with any tcp stream that implements `futures::{AsyncRead, AsyncWrite}`.
- All dependencies are async-ecosystem independent.
- Fast enough.

# Upcoming
- Transfer-encoding
- Completely correct handling of HTTP protocol
- Request/Response logging

# License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
