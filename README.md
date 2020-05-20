# tophat
[![CI](https://github.com/hwchen/tophat/workflows/ci/badge.svg)](https://github.com/hwchen/tophat/actions?query=workflow%3Aci)

A small, pragmatic, and flexible async HTTP server library. Currently in beta.

The goal is to be low-level and small enough to work with different async runtimes and not dictate user architecture, while having enough convenience functions to still easily build a REST api. More library than framework.

Also, this:
```rust
async fn handler<W>(_req: Request, resp_wtr: ResponseWriter<W>) -> Result<ResponseWritten, Glitch>
    where W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    // Default `send` is 200 OK
    let done = resp_wtr.send()?;
    // Do things here after resp is written, if you like
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
- Router `features = ["router"]`, very minimal.
- Cors `features = ["cors"]`.
- Identity `features = ["identity"]`.
- "Middleware" capabilities by using functions in front of router.
- Convenient error/response handling using `Glitch` and `GlitchExt`, to conveniently chain onto both `Result` and `Option`.

Correct handling of the HTTP protocol is a priority.

Upcoming features:
- Examples, with integrations.
- Client?

Long term:
- HTTP/2

# Philosophy

I wouldn't consider this a batteries-included framework which tries to make every step easy. There are conveniences, but overall tophat is pretty minimal. For those who don't like boilerplate, another framework would probably work better. Users of tophat need to be familiar async runtimes, setting up a TCP stream, `Arc`, traits, generics, etc. Tophat won't hold your hand.

In exchange, tophat provides more transparency and more control. Tophat won't dictate how to structure your app, it should play nicely with your architecture.

And if you want to know what tophat is doing under the hood, the code is meant to be simple and straightforward (Hopefully this also leads to better compile times!).

# Thanks
Especially to [async-h1](https://github.com/http-rs/async-h1), whose eye for structure and design I appreciate, and whose code base tophat is built from.
And to [hyper](https://github.com/hyperium/hyper), whose devotion to performance and correctness is inspiring, and whose basic http libraries tophat has incorporated.

# License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
