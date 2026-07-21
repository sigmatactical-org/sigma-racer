# M7 bring-up (Verdin iMX8M Plus)

End-to-end heterogeneous multiprocessing: U-Boot `bootaux`, Linux
`remoteproc`, virtio RPMsg to `sigma-racer-vehicle`, and FLEXCAN1 safety bus
to the ECU.

## Architecture

| Domain | Bus / IPC | Hardware |
|--------|-----------|----------|
| **M7** (`sigma-racer-sidearm`) | FLEXCAN1 → ECU safety bus | Verdin SODIMM 20/22 |
| **M7 → A53** | RPMsg `sigma-m7-signals` | MU1 + virtio vrings |
| **Linux** | FLEXCAN2 (`can1`) infotainment | Verdin SODIMM 24/26 |
| **M7 debug** | LPUART4 @ 115200 | Verdin SODIMM 151/153 |

## Memory layouts

| Layout | Feature | Link regions | U-Boot `bootaux` |
|--------|---------|--------------|------------------|
| **DDR** (default) | *(none)* | code `0x8000_0000`, RAM `0x8020_0000` | `bootaux 0x80000000` |
| **ITCM** | `memory-itcm` | code `0x0000_0000`, RAM `0x2000_0000` | `bootaux 0` |

DDR shared-memory carve-out (Linux DT + firmware):

| Region | Address | Size |
|--------|---------|------|
| M7 firmware | `0x8000_0000` | 16 MiB (`m7_reserved`) |
| vring0 | `0x5500_0000` | 32 KiB |
| vring1 | `0x5500_8000` | 32 KiB |
| rpmsg pool | `0x5540_0000` | 1 MiB |
| resource table copy | `0x550F_F000` | 4 KiB |

## Build

```bash
# Host — CAN contract + wire codec
cargo test --target x86_64-unknown-linux-gnu

# M7 UART boot proof
cargo build --release --no-default-features --features bringup

# Production Embassy firmware (FlexCAN + RPMsg + resource table)
cargo build --release --no-default-features --features firmware

# Debian package (ELF + sigma-racer-sidearm.service) for updates / images
./scripts/package-deb.sh
```

Artifacts:

- `target/thumbv7em-none-eabihf/release/sigma-racer-sidearm` (ELF, `.resource_table` section)
- `target/thumbv7em-none-eabihf/release/sigma-racer-sidearm-bringup`
- `dist/sigma-racer-sidearm-firmware_0.1.0-r0_all.deb`

## U-Boot `bootaux` (DDR bring-up binary)

```text
ext4load mmc 2:2 ${loadaddr} /sigma-racer-sidearm-bringup.bin
cp.b ${loadaddr} 0x80000000 ${filesize}
bootaux 0x80000000
```

Expected on LPUART4 @ 115200 8N1:

```text
sigma-racer-sidearm bringup
M7 alive on LPUART4 (115200 8N1)
```

## Linux remoteproc (production)

1. Apply `sigma-racer-wingman-hmp.dtbo` (disables Linux `flexcan1` + `uart4`).
2. Kernel cmdline: `clk-imx8mp.mcore_booted=1`
3. Install firmware: `/lib/firmware/sigma-racer-sidearm.elf`
4. Start: `sigma-racer-sidearm.service` or manually:

```bash
echo sigma-racer-sidearm.elf > /sys/class/remoteproc/remoteproc0/firmware
echo start > /sys/class/remoteproc/remoteproc0/state
```

5. Verify RPMsg endpoint:

```bash
ls /sys/bus/rpmsg/devices/
# sigma-m7-signals → /dev/rpmsgN
```

6. Run vehicle daemon with `VEHICLE_SOURCE=rpmsg` (default in Wingman image).

## Platform init (`sigma_racer_sidearm::hw`)

Runs from `#[cortex_m_rt::pre_init]` before main:

1. **Cache** — I/D-cache disabled for safe peripheral MMIO.
2. **Clock** — CCM ungate for UART4 and FLEXCAN1.
3. **RDC** — M7 domain access to CAN1 / GPIO.
4. **FLEXCAN1** — 1 Mbit/s classic CAN, Verdin CAN_1 pins.
5. **RPMsg** — NXP resource table + virtio `sigma-m7-signals` publisher.

## Heartbeat

M7 emits `0x080` @ 50 Hz (sequence byte) so the ECU knows the safety core is
alive independent of Linux.

## Helper script

`scripts/load-m7.sh` — build, `objcopy`, and print U-Boot commands.
