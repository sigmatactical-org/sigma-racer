#!/usr/bin/env bash
# Build M7 firmware and pack sigma-racer-sidearm-firmware_*.deb
# (ELF + remoteproc systemd unit). Matches the Yocto recipe layout.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SERVICE_SRC="${SIDEARM_SERVICE_SRC:-$ROOT/../sigma-racer-wingman/meta-sigma-racer-wingman/recipes-sigma-racer-wingman/sigma-racer-sidearm/sigma-racer-sidearm-firmware/sigma-racer-sidearm.service}"
VERSION="${SIDEARM_DEB_VERSION:-0.1.0-r0}"
ARCH="${SIDEARM_DEB_ARCH:-all}"
OUT_DIR="${SIDEARM_DEB_OUT:-$ROOT/dist}"
PKG_NAME="sigma-racer-sidearm-firmware"
DEB_NAME="${PKG_NAME}_${VERSION}_${ARCH}.deb"

if [[ ! -f "$SERVICE_SRC" ]]; then
  echo "error: systemd unit not found: $SERVICE_SRC" >&2
  exit 1
fi

cd "$ROOT"
cargo build --release --no-default-features --features firmware --bin sigma-racer-sidearm
# In the monorepo the build output lands in the workspace-shared target dir, not
# $ROOT/target — resolve it from cargo rather than assuming a standalone layout.
TARGET_DIR="${CARGO_TARGET_DIR:-$(cargo metadata --format-version 1 --no-deps \
  | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')}"
ELF="${TARGET_DIR}/thumbv7em-none-eabihf/release/sigma-racer-sidearm"
test -f "$ELF"

STAGE=$(mktemp -d)
trap 'rm -rf "$STAGE"' EXIT

mkdir -p \
  "$STAGE/lib/firmware" \
  "$STAGE/usr/lib/systemd/system" \
  "$STAGE/usr/lib/systemd/system-preset" \
  "$STAGE/DEBIAN"

install -m 0644 "$ELF" "$STAGE/lib/firmware/sigma-racer-sidearm.elf"
install -m 0644 "$SERVICE_SRC" "$STAGE/usr/lib/systemd/system/sigma-racer-sidearm.service"
printf 'enable sigma-racer-sidearm.service\n' \
  >"$STAGE/usr/lib/systemd/system-preset/98-sigma-racer-sidearm-firmware.preset"

SIZE=$(du -sk "$STAGE" | cut -f1)

cat >"$STAGE/DEBIAN/control" <<EOF
Package: ${PKG_NAME}
Version: ${VERSION}
Architecture: ${ARCH}
Maintainer: Sigma Embedded <embedded@sigma.local>
Section: base
Priority: optional
Depends: bash
Installed-Size: ${SIZE}
Homepage: https://github.com/sigmatactical-org/sigma-racer-sidearm
Description: Sigma Racer M7 safety-core firmware (sigma-racer-sidearm)
 Cortex-M7 remoteproc firmware and systemd unit that loads
 /lib/firmware/sigma-racer-sidearm.elf on i.MX 8M Plus (no-op on
 hosts without remoteproc0, e.g. QEMU virt).
EOF

cat >"$STAGE/DEBIAN/postinst" <<'EOF'
#!/bin/sh
set -e
if systemctl >/dev/null 2>/dev/null; then
	if [ -z "${D:-}" ]; then
		systemctl daemon-reload
		systemctl preset sigma-racer-sidearm.service || true
		systemctl --no-block restart sigma-racer-sidearm.service || true
	else
		systemctl --root="$D" enable sigma-racer-sidearm.service || true
	fi
fi
EOF

cat >"$STAGE/DEBIAN/prerm" <<'EOF'
#!/bin/sh
[ "$1" != "upgrade" ] || exit 0
set -e
if systemctl >/dev/null 2>/dev/null; then
	if [ -z "${D:-}" ]; then
		systemctl stop sigma-racer-sidearm.service || true
		systemctl disable sigma-racer-sidearm.service || true
	fi
fi
EOF

chmod 0755 "$STAGE/DEBIAN/postinst" "$STAGE/DEBIAN/prerm"

mkdir -p "$OUT_DIR"
dpkg-deb --root-owner-group --build "$STAGE" "$OUT_DIR/$DEB_NAME"
echo "Wrote $OUT_DIR/$DEB_NAME"
dpkg-deb -I "$OUT_DIR/$DEB_NAME"
dpkg-deb -c "$OUT_DIR/$DEB_NAME"
