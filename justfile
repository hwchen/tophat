test:
    cargo watch -x 'test -- --nocapture'

bench:
    cargo watch -x 'run --release --example bench'

basic:
    cargo watch -x 'run --release --example basic'
