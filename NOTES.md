# Implementation Notes
Probably going to follow `async-h1` overall structure to keep it simple first. Just try to use the components from hyper, which have fewer deps.

The inspiration: talking about using Sinks instead of Streams for sending, and how that can affect observability of responses.
https://github.com/hyperium/hyper/issues/2181
https://users.rust-lang.org/t/async-interviews/35167/33

`hyper/http` and `hyper/http-body` and `http/httpparse` for implementing the http basics
https://github.com/hyperium/http-body/blob/master/src/lib.rs

Some other implementation examples from async-std: `http-types`, `async-h1`
https://github.com/http-rs/http-types/blob/master/src/body.rs

Other implementation notes from hyper: `body.rs`
https://docs.rs/hyper/0.13.5/src/hyper/body/body.rs.html#84-87

