watch:
    cargo watch -x 'check --all-features --examples --tests'

test filter='':
    cargo watch -x 'test {{filter}}'

test-anyhow:
    cargo watch -x 'test --features=anyhow -- --nocapture'

bench:
    RUST_LOG=info cargo watch -x 'run --release --example bench'

basic:
    RUST_LOG=info cargo watch -x 'run --release --example basic'

routing:
    RUST_LOG=info cargo watch -x 'run --release --example routing --features="router"'

routing_2:
    RUST_LOG=info cargo watch -x 'run --release --example routing_2 --features="router"'

identity:
    RUST_LOG=info cargo watch --ignore .cookie -x 'run --release --example identity --features="router identity"'

identity-login:
    curl -v --cookie .cookie --cookie-jar .cookie --location localhost:9999/login/test

identity-hello:
    curl -v --cookie .cookie localhost:9999/

identity-logout:
    curl -v --cookie .cookie --cookie-jar .cookie --location localhost:9999/logout

clippy:
    cargo watch -x '+nightly clippy --all-features -- -D warnings -Z unstable-options'
