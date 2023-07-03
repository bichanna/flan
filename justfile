test:
    cargo test -- --nocapture

check:
    cargo clippy --verbose

build:
    cargo build --verbose

release:
    cargo build --release
