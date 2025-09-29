.PHONY: build test check test-unit test-integration run clean lint fmt \
 		fmt-check setup update-test-environment accept-snapshots \
		cleanup-containers

build:
	cargo build

test: test-unit test-integration

test-unit:
	cargo test --lib

test-integration:
	cargo test --lib --features integration-tests integration_tests -- --test-threads=1

run:
	cargo run

check: lint fmt-check test

lint:
	cargo clippy -- -D warnings

fmt:
	cargo fmt

fmt-check:
	cargo fmt --check

setup:
	git config core.hooksPath .githooks

clean:
	cargo clean

# Fetches the latest data from the production backend and updates the test
# environment files in `src/test_support/environment`.
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
