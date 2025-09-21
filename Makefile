.PHONY: build test run clean

build:
	cargo build

test:
	cargo test --lib

test-integration:
	cargo test --test integration_tests -- --test-threads=1

run:
	cargo run

clean:
	cargo clean