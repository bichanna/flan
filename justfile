test:
    cargo test --features debug -- --nocapture 

check:
    cargo clippy --verbose

build:
    cargo build --verbose --features debug

release:
    cargo build --release

debug:
    cargo run --features "debug"
