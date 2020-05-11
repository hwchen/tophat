watch:
    cargo watch -x 'check --examples --tests'

test:
    cargo watch -x 'test -- --nocapture'

bench:
    cargo watch -x 'run --release --example bench'

basic:
    RUST_LOG=info cargo watch -x 'run --release --example basic'

routing:
    RUST_LOG=info cargo watch -x 'run --release --example routing'

routing_2:
    RUST_LOG=info cargo watch -x 'run --release --example routing_2'

clippy:
    cargo watch -x '+nightly clippy -- -D warnings -Z unstable-options'
