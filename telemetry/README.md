# sigma-racer-telemetry

[![CI](https://github.com/sigmatactical-org/sigma-racer-telemetry/actions/workflows/ci.yml/badge.svg)](https://github.com/sigmatactical-org/sigma-racer-telemetry/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)
[![MSRV](https://img.shields.io/badge/MSRV-1.97.0-blue.svg)](https://www.rust-lang.org)

VSS vehicle state, M7 CAN bridge, and NDJSON IPC for Sigma Racer cockpit services.

## Crate

- **`sigma-racer-telemetry`** — `VehicleState`, Unix-socket telemetry protocol, publisher helpers, and `TelemetryClient` for subscribers.

## Consumers

- **sigma-racer-vehicle** — CAN/sim → VSS → `/run/sigma-racer-wingman/vehicle.sock`
- **sigma-racer-cluster** — Slint dashboard subscriber

## Test

```bash
cargo test
```

## Brand & artwork

© Sigma Tactical Group. **All rights reserved.**

The Sigma Tactical Group name, logos, marks, artwork, and visual identity are **proprietary**. They are not covered by this repository's source-code license. See [BRANDING.md](BRANDING.md).

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
