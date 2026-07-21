# Sigma Racer Cluster

[![CI](https://github.com/sigmatactical-org/sigma-racer-cluster/actions/workflows/ci.yml/badge.svg)](https://github.com/sigmatactical-org/sigma-racer-cluster/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-GPL--3.0--only-blue.svg)](#license)
[![MSRV](https://img.shields.io/badge/MSRV-1.97.0-blue.svg)](https://www.rust-lang.org)

Production instrument cluster application for the **Sigma Racer Wingman** motorcycle
display — ships as the `sigma-racer-cluster` binary on i.MX 8M Plus / i.MX 95 class hardware.

| Binary | Role |
|--------|------|
| `sigma-racer-cluster` | Full-screen Slint cluster UI (CAN-FD telemetry from M7 safety core) |

## Quick start

Requires sibling checkouts under `embedded/`:

```
embedded/
├── sigma-instrumentation/   # UI library + telemetry crate
└── sigma-racer-cluster/     # this repo
```

```bash
# Production binary (idle telemetry — same as embedded target)
cargo run --bin sigma-racer-cluster

# Panel-accurate local testing (800×480, matches imx8mp / QEMU virt)
cd ../sigma-instrumentation && cargo virt
```

## Telemetry / CAN test sources

The live source is chosen at runtime by `CLUSTER_TELEMETRY_SOURCE`. All sources
decode into `VehicleState`, map to `ClusterTelemetry`, and call
`sigma_instrumentation::apply_telemetry` — the UI never sees raw CAN.

| `CLUSTER_TELEMETRY_SOURCE` | Source |
|----------------------------|--------|
| `ipc` *(default)* | Subscribe to `sigma-racer-vehicle` over its Unix socket (production behaviour). |
| `replay` | Replay a `candump -L` log through the DBC decoder with original timing, looping. Uses the baked-in [`testdata/sample-ride.log`](testdata/sample-ride.log) unless `CLUSTER_REPLAY_LOG` points elsewhere. |
| `can` / `socketcan` | Read live frames off a SocketCAN interface (`CLUSTER_CAN_IFACE`, default `vcan0`). Requires `--features can-socket`. |

```bash
# Replay the baked sample ride — no hardware, no daemon
CLUSTER_TELEMETRY_SOURCE=replay cargo run

# Replay your own capture
CLUSTER_REPLAY_LOG=/path/to/ride.log CLUSTER_TELEMETRY_SOURCE=replay cargo run

# Live SocketCAN off vcan0 (bring the bus up first)
sudo scripts/vcan-up.sh vcan0
CLUSTER_TELEMETRY_SOURCE=can cargo run --features can-socket
# …then feed it, e.g.:  canplayer -I testdata/sample-ride.log vcan0=can1
```

Regenerate the baked sample from the scripted ride:

```bash
cargo run --example gen_sample_log > testdata/sample-ride.log
```

> The `can` source consumes whatever populates the bus. On the vehicle the M7
> safety core owns the CAN traffic and gateways it onto the Linux-visible bus;
> for the bench, `vcan0` + `canplayer`/`cansend` (or the sample replay) stand in.

## Embedded build (Wingman)

The Yocto recipe builds **`sigma-racer-cluster`** from this crate:

```bash
bitbake sigma-racer-cluster
```

| Item | Value |
|------|-------|
| Binary | `/usr/bin/sigma-racer-cluster` |
| systemd | `cluster-ui.service` |
| Environment | `/etc/sigma-racer-wingman/ui.env` |

Full distribution docs: [`sigma-racer-wingman`](../sigma-racer-wingman/README.md).

## Requirements

- Rust 1.97+ (Yocto meta-rust scarthgap)
- Slint 1.17.1 (pinned)
- [`sigma-instrumentation`](../sigma-instrumentation/) workspace (library + telemetry)

## Brand & artwork

© Sigma Tactical Group. **All rights reserved.**

The Sigma Tactical Group name, logos, marks, artwork, and visual identity are **proprietary**. They are not covered by this repository's source-code license. See [BRANDING.md](BRANDING.md).

## License

**GPL-3.0-only** — see [LICENSE](LICENSE).

This repository is the exception to the Sigma Racer workspace's usual
MIT OR Apache-2.0 licensing: it links [Slint](https://slint.dev) under
Slint's GPL-3.0-only option, which permits deployment on embedded
hardware without a commercial Slint license.

### Licensing boundary

- **Logic lives downstream.** CAN decode, protocols, and vehicle state
  belong in the permissive (MIT OR Apache-2.0) crates —
  `sigma-racer-telemetry`, `sigma-diagnostics`, and the
  `sigma-instrumentation` library. This crate is the thin GPL shell
  that assembles them into the shipped cluster binary. If a change here
  starts growing reusable logic, move it down first.
- **Contributions are dual-licensed.** By contributing to this
  repository you agree your contribution is licensed MIT OR Apache-2.0
  (as elsewhere in the Sigma Racer workspace), in addition to being
  distributed here under GPL-3.0-only. This keeps the maintainers free
  to move code into the permissive crates.
- **No proprietary artwork.** Sigma Tactical Group brand assets are not
  included in this repository (see [BRANDING.md](BRANDING.md)); the UI
  is code-drawn. Do not add proprietary artwork to this GPL-licensed
  tree.

### Shipping obligations

Conveying a vehicle with this software installed is distribution under
GPLv3:

- **Corresponding source** for the combined cluster binary must
  accompany the device or be available by written offer.
- **Installation Information (GPLv3 §6):** on a consumer product,
  owners must be able to install modified versions. The signed-firmware
  / RAUC OTA design must include an owner-unlock path for the cluster
  image.
