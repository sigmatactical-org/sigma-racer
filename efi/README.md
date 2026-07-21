# Sigma Racer EFI

[![CI](https://github.com/sigmatactical-org/sigma-racer-efi/actions/workflows/ci.yml/badge.svg)](https://github.com/sigmatactical-org/sigma-racer-efi/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)
[![MSRV](https://img.shields.io/badge/MSRV-1.97.0-blue.svg)](https://www.rust-lang.org)

Rust engine control firmware for the [microRusEFI](https://www.shop.rusefi.com/shop/p/microrusefi-assembled-ecu-development-module) ECU, built on [Embassy](https://embassy.dev/) and conceptually ported from [rusEFI](https://github.com/rusefi/rusefi).

Licensed under **MIT OR Apache-2.0** (see `LICENSE-MIT` and `LICENSE-APACHE`). rusEFI itself is GPL; this project reimplements algorithms and maps hardware from public board documentation — it does not incorporate rusEFI source code.

## Architecture

The core is **engine-agnostic**. Board support (microRusEFI pins, ADC scaling) lives separately from engine profiles (cylinder count, firing order, trigger wheel, rev limits).

```
src/
├── config.rs          # EngineConfig, injection/ignition modes, firing presets
├── engines/
│   ├── profile.rs     # EngineProfile (shared struct)
│   └── yamaha_cp3.rs  # Yamaha CP3 triple
├── defaults.rs        # MRE wiring (maps cylinder index → outputs)
└── pins.rs            # STM32 pin map
```

Pick an engine at **compile time** with Cargo features. Add a new engine by creating `src/engines/your_engine.rs` and a matching feature flag.

### Built-in engine profiles

| Feature | Engine | Cylinders |
|---------|--------|-----------|
| `engine-yamaha-cp3` | Yamaha CP3 (MT-09, XSR900, R9) | 3 |

## Hardware

| Item | Value |
|------|-------|
| Board | microRusEFI (rev 0.6.x wiring) |
| MCU | STM32F767VI (verify against your BOM) |
| Smart driver | TLE8888 (injectors, low-side outputs) |
| ETB driver | TLE9201 |
| rusEFI board profile | `meta-info-mre_f7.env` |

Pin assignments live in `src/pins.rs`, derived from rusEFI's `board_configuration.cpp` and `connectors/main.yaml`.

## Build

Install the embedded target once:

```bash
rustup target add thumbv7em-none-eabihf
```

Build firmware:

```bash
cd embedded/sigma-racer-efi
cargo build --features firmware,engine-yamaha-cp3 --release --target thumbv7em-none-eabihf
```

Flash (requires [probe-rs](https://probe.rs/) and a SWD probe):

```bash
probe-rs run --chip STM32F767VI target/thumbv7em-none-eabihf/release/sigma-racer-efi
```

Run host unit tests:

```bash
cargo test
```

## Current status

**Stage 1 — characterization data logger** (mule runbook Phase 1):

- 216 MHz sysclk from HSI (works on any board; switch the PLL source to
  HSE for CAN-grade accuracy once the fitted crystal is verified)
- 1 MHz embassy-time tick → µs timestamps on every record
- `MreBoard::init` splits `Peripherals` in one place: LEDs, ADC1 sweep
  channels, crank (PC6/EXTI6) and cam (PA5/EXTI5) inputs
- Analog sweep at 100 Hz — VBatt / CLT / IAT / TPS-MAP / AN volt 1–2,
  scaled with the rusEFI front-end constants, published via `Watch` and
  streamed as `DL,S` records at 10 Hz
- Crank/cam edge capture with per-line interval stats streamed as `DL,T`
  records — the gap-ratio column exposes the missing-tooth signature
  without assuming wheel geometry (characterization-grade; the
  input-capture decoder replaces this for engine control)
- Blinks **running** (PE4) / **comms** (PE2); **critical** (PE3)
  fast-blink = safe state (invalid profile or task-spawn failure)

Record formats (defmt over RTT, parse with `probe-rs` + `defmt-print`):

```text
DL,S,<t_us>,<vbatt_v>,<clt_c>,<iat_c>,<tps_map_v>,<an1_v>,<an2_v>
DL,T,<C|V>,<count>,<t_us>,<period_us>,<gap_ratio_x100>
DL,X,dropped_edges,<n>
```

## Roadmap (rusEFI parity)

Priority order for porting rusEFI subsystems into Rust:

1. **Trigger** — crank/cam decoding on PC6 / PA5, tooth scheduler
2. **TLE8888** — SPI1 driver for injectors and low-side outputs
3. **Fuel** — speed-density, VE tables, sequential scheduling
4. **Ignition** — coil charge/fire on PD1–PD4
5. **Sensors** — ADC scan (CLT, IAT, MAP, VBatt), NTC math
6. **CAN / USB** — tuning interface (rusEFI protocol or new)
7. **ETB** — TLE9201 drive-by-wire
8. **Storage** — onboard SD (SPI2) for logs and tune persistence

## References

- [microRusEFI shop listing](https://www.shop.rusefi.com/shop/p/microrusefi-assembled-ecu-development-module)
- [rusEFI repository](https://github.com/rusefi/rusefi)
- [microRusEFI hardware repo](https://github.com/rusefi/hw_microRusEfi)
- [rusEFI microRusEFI wiring wiki](https://wiki.rusefi.com/Hardware-microRusEfi-wiring)
- [Embassy STM32F767 docs](https://docs.embassy.dev/embassy-stm32/git/stm32f767vi/index.html)
- [Yamaha CP3 engine overview](https://motofomo.com/yamaha-cp3-history-models/)

## Brand & artwork

© Sigma Tactical Group. **All rights reserved.**

The Sigma Tactical Group name, logos, marks, artwork, and visual identity are **proprietary**. They are not covered by this repository's source-code license. See [BRANDING.md](BRANDING.md).

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
