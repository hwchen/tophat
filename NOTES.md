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

# Error handling
Try to handle local fails in each module, bubbling up those failures so they can be handled in the root module. Try to keep the handlers for those failures in each modules also, and output a response, because internal failures should generally be handled by issuing a bad request or internal server error. Catastrophic system failure is a bug. Basically, `accept` should never fail.

# HTTP RFCs to read
[Message Syntax and Routing](https://tools.ietf.org/html/rfc7230)
- [Message Body Length](https://tools.ietf.org/html/rfc7230#section-3.3.3)
[Original](https://tools.ietf.org/html/rfc2616)

# URI handling
Looks like https://tools.ietf.org/html/rfc2616#section-5.2 is the section to look at. The question is whether hyper just ignores the host, like the section says is possible?

Section 19.6.1.1 (requirements for HTTP/1.1 server):

- server must report 400 if no Host header
- server must accept absolute URI
- https://tools.ietf.org/html/rfc2616#section-19.6.1.1

absolute URI: https://tools.ietf.org/html/rfc2396#section-3

absoluteURI is the "whole" url, absolute path is everything after the authority excluding query.

Check what happens with query strings.

Looks like hyper just ignores: https://github.com/hyperium/hyper/blob/master/src/proto/h1/role.rs#L102

```rust
subject = RequestLine(
    Method::from_bytes(req.method.unwrap().as_bytes())?,
    req.path.unwrap().parse()?,
);
```

This lets them just accept absolute paths also.

async-h1 formats with a scheme and authority onto path, I think this is incorrect.

# Philosophical notes:

Designed using language constructs to build your app, instead of creating another layer of abstraction. So using streams and asyncread and write instead of service architecture when possible. The language already gives you tools which are very powerful and composable, so defer to those when possible.

And instead of services for backend (like timeout and compression) just use async io traits and streams. Just need to provide hooks for them.
