.PHONY: help
help:
	@echo "Targets: \n" \
	"    test       - runs all test \n" \
	"    build      - build debug executable \n" \
	"    build-prod - build production-ready executable"

.PHONY: test
test:
	cargo test --features debug

.PHONY: build
build:
	cargo build

.PHONY: build-prod
build-prod:
	cargo build --release
