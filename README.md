# bStats Data Processor

A high-performance data processor for [bStats], responsible for handling the
data sent by bStats' Metrics classes.

## Development

Checkout the README of the [bStats] repository for instructions on how to run
the complete bStats stack locally.

However, most of the time you do not need to run the complete stack to make
changes to this project. Simply running the test suite is more convenient and
sufficient in most cases.

### Prerequisites

- Linux. WSL2 is recommended for Windows. MacOS might work but is untested
- Rust
- Docker (the test suite uses [testcontainers] to run a Redis cluster)

### Important Commands

The `Makefile` contains several useful commands. The most important ones are:

- `make setup` - Installs a pre-commit hook to run the test suite before each
  commit
- `make check` - Runs the test suite and lints the code
- `make fmt` - Formats the code

## Environment Variables

The following environment variables are used by the application:

### Required Variables

- **`REDIS_CLUSTER__URLS`** - Comma-separated list of Redis cluster URLs for data storage
  - Example: `redis://localhost:6379,redis://localhost:6380,redis://localhost:6381`

### Optional Variables

- **`HOST`** - Server bind address (default: `0.0.0.0`)
- **`PORT`** - Server port number (default: `8080`)
- **`WORKERS`** - Number of worker processes for the HTTP server (default: auto-detected)
- **`GEOIP_DATABASE_PATH`** - Path to the GeoIP database file (default: `GeoLite2-Country.mmdb`)
- **`BEHIND_PROXY`** - Set to `true` if behind a proxy. Uses `forwarded` and `x-forwarded-for` for IP resolution (default: `false`)
- **`BEHIND_CLOUDFLARE_PROXY`** - Set to `true` if behind a Cloudflare proxy. Uses `cf-connecting-ip` for IP resolution (default: `false`)
- **`WORD_BLOCKLIST`** - JSON array of words to block in submissions (default: `[]`)
  - Example: `["badword1", "badword2"]`

[bstats]: https://github.com/Bastian/bstats
[testcontainers]: https://testcontainers.com/
