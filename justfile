watch:
    cargo watch -x 'check --examples --tests'

test:
    cargo watch -x 'test -- --nocapture'

bench:
    cargo watch -x 'run --release --example bench'

basic:
    cargo watch -x 'run --release --example basic'

clippy:
    cargo watch -x '+nightly clippy -- -D warnings -Z unstable-options'
