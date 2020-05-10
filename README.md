# tophat
[![CI](https://github.com/hwchen/tophat/workflows/ci/badge.svg)](https://github.com/hwchen/tophat/actions?query=workflow%3Aci)

An async HTTP server library. Currently in alpha.

The goal is to be low-level and small enough to work with different async runtimes and not dictate user architecture, while having enough convenience functions to still easily build a REST api.

Also, this:
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
- HTTP/1.1
- Works with any tcp stream that implements `futures::{AsyncRead, AsyncWrite}`.
- All dependencies are async-ecosystem independent.
- Not meant to be a framework; minimal abstraction.
- #[deny(unsafe_code)]
- Fast enough.

Correct handling of the HTTP protocol is a priority.

Upcoming features:
- Router. Service abstraction?
- Convenience functions for building Responses.
- Json feature.
- Client.
- Lots of examples.
- static file serving? (have to investigate something like `blocking`).

Long term:
- HTTP/2

# Thanks
Especially to [async-h1](https://github.com/http-rs/async-h1), whose eye for structure and design I appreciate, and whose code base tophat is built from.
And to [hyper](https://github.com/hyperium/hyper), whose devotion to performance and correctness is inspiring, and whose basic http libraries tophat has incorporated.

# License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
