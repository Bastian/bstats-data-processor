.PHONY: build test run clean lint fmt setup cleanup-containers

build:
	cargo build

setup:
	git config core.hooksPath .githooks

check: lint fmt test

test: test-unit test-integration

test-unit:
	cargo test --lib

test-integration:
	cargo test --lib --features integration-tests integration_tests -- --test-threads=1

run:
	cargo run

clean:
	cargo clean

lint:
	cargo clippy -- -D warnings

fmt:
	cargo fmt --check


update-test-environment:
	cd src/test_support/environment && ./import

# Accept all new snapshot files created by the tests
accept-snapshots:
	# Find all files with the .snap.new extension and rename them to .snap
	# Also remove the `assertion_line: <number>` metadata line added by insta
	# See https://github.com/mitsuhiko/insta/pull/218
	find . -type f -name '*.snap.new' | while read -r file; do \
		mv "$$file" "$${file%.new}"; \
		sed -i '/^assertion_line: [0-9]\+$$/d' "$${file%.new}"; \
	done

# Remove orphaned testcontainers (useful after Ctrl+C during tests)
cleanup-containers:
	docker ps -a --filter "label=bstats.test=redis-cluster" -q | xargs -r docker rm -f
