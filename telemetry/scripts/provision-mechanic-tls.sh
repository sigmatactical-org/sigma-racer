#!/usr/bin/env bash
# Copy Mechanic mTLS client material from a Wingman PKI directory.
#
# Usage:
#   ./scripts/provision-mechanic-tls.sh [pki-dir]
#
# Default pki-dir: ./telemetry-tls (output of gen-telemetry-tls.sh)
# Installs to ~/.config/sigma-racer-mechanic/tls/ unless
# SIGMA_RACER_MECHANIC_CONFIG_DIR is set.
set -euo pipefail

PKI="${1:-./telemetry-tls}"
DEST="${SIGMA_RACER_MECHANIC_CONFIG_DIR:-${HOME}/.config/sigma-racer-mechanic}/tls"

for f in ca.pem client.pem client.key; do
  if [[ ! -f "${PKI}/${f}" ]]; then
    echo "missing ${PKI}/${f} — run scripts/gen-telemetry-tls.sh first" >&2
    exit 1
  fi
done

mkdir -p "${DEST}"
chmod 700 "${DEST}"
install -m 0644 "${PKI}/ca.pem" "${DEST}/ca.pem"
install -m 0644 "${PKI}/client.pem" "${DEST}/client.pem"
install -m 0600 "${PKI}/client.key" "${DEST}/client.key"

if [[ -f "${PKI}/server.pin" ]]; then
  echo "Optional certificate pin (recommended for production):"
  echo "  export TELEMETRY_TLS_SERVER_PIN=$(tr -d '\n' < "${PKI}/server.pin")"
fi

echo "Mechanic TLS material installed in ${DEST}"
