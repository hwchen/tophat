# 2020-05-19, v0.2.0
## Features
- `ResponseWriter` now holds a `Response`, so it's not need to create one separately.
- Convenience methods on `ResponseWriter`.
- `Glitch` and `GlitchExt` for error management to error response.
- `ResponseWritten` no longer creatable by user.
- Router now behind feature gate.
- Cors, feature gated.
- Identity, feature gated.
- "Middleware" philosophy confirmed. (no specific framework for it)
- Beginning of docs.


## Internal
- remove `mime` crate.
- pub use `http` crate.
- remove more unwraps.
- ci on all features
- remove clippy on stable (nightly has different lints)
- anyhow was added then removed.
