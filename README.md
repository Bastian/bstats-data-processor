# bStats Data Processor (WORK IN PROGRESS)

## Description

This repository contains experimental code to process data sent by the bStats
Metrics classes. It's written in Rust to for best performance.

**IT IS CURRENTLY A WORK IN PROGRESS AND NOT YET FUNCTIONAL AND MAYBE NEVER WILL
BE**

The current production can be found in the [bstats-backend] repo.

## Environment Variables

The following environment variables are used by the application:

| Variable                  | Description                                                                               | Default                 |
| ------------------------- | ----------------------------------------------------------------------------------------- | ----------------------- |
| `GEOIP_DATABASE_PATH`     | Path to the GeoIP database file                                                           | `GeoLite2-Country.mmdb` |
| `BEHIND_PROXY`            | Set to `true` if behind a proxy. Uses `forwarded` and `x-forwarded-for` for ip resolution | `false`                 |
| `BEHIND_CLOUDFLARE_PROXY` | Set to `true` if behind a Cloudflare proxy. Uses `cf-connecting-ip` for ip resolution     | `false`                 |

[bstats-backend]: https://github.com/Bastian/bstats-backend
