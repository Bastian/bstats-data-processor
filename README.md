# bStats Data Processor (WORK IN PROGRESS)

## Description

This repository contains experimental code to process data sent by the bStats
Metrics classes. It's written in Rust to for best performance.

**IT IS CURRENTLY A WORK IN PROGRESS AND NOT YET FUNCTIONAL AND MAYBE NEVER WILL
BE**

The current production can be found in the [bstats-backend] repo.

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

[bstats-backend]: https://github.com/Bastian/bstats-backend
