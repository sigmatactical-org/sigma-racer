#!/usr/bin/env bash
# Emit U-Boot commands to load an M7 binary built by sigma-racer-sidearm.
set -euo pipefail

PROFILE="${1:-release}"
FEATURES="${2:-bringup}"
BIN="sigma-racer-sidearm-bringup"
TARGET="thumbv7em-none-eabihf"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ELF="$ROOT/target/$TARGET/$PROFILE/$BIN"
FLAT="${ELF}.bin"

if [[ "$FEATURES" == *"memory-itcm"* ]]; then
  LOAD_ADDR="0"
else
  LOAD_ADDR="0x80000000"
fi

echo "Building $BIN (profile=$PROFILE, features=$FEATURES) ..."
cargo build --manifest-path "$ROOT/Cargo.toml" \
  --no-default-features \
  --features "$FEATURES" \
  ${PROFILE:+--$PROFILE}

rust-objcopy -O binary "$ELF" "$FLAT"
SIZE=$(wc -c < "$FLAT")

cat <<EOF

# Copy $FLAT (${SIZE} bytes) to the board, then in U-Boot:

ext4load mmc 2:2 \${loadaddr} /$(basename "$FLAT")
cp.b \${loadaddr} ${LOAD_ADDR} \${filesize}
bootaux ${LOAD_ADDR}

# Serial: LPUART4 @ 115200 8N1 (Verdin M7 debug UART)

EOF
