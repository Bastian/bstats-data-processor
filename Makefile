.PHONY: build test run clean lint fmt

build:
	cargo build

check: lint fmt test

test: test-unit test-integration

test-unit:
	cargo test --lib

test-integration:
	cargo test --test integration_tests -- --test-threads=1

run:
	cargo run

clean:
	cargo clean

lint:
	cargo clippy -- -D warnings

fmt:
	cargo fmt --check

update-test-environment:
	cd tests/environment && ./import
