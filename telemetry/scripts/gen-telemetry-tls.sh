#!/usr/bin/env bash
# Generate a private PKI for Sigma Racer Wingman telemetry (TLS 1.3 + mTLS).
#
# Usage:
#   ./scripts/gen-telemetry-tls.sh [output-dir] [wingman-ip-or-dns]
#
# Creates:
#   ca.pem       — trust anchor (install on Wingman + Mechanic)
#   server.pem   — Wingman relay certificate (CN=wingman-telemetry + SAN)
#   server.key   — Wingman relay private key
#   client.pem   — Mechanic client certificate (CN=sigma-racer-mechanic)
#   client.key   — Mechanic client private key
#   server.pin   — SHA-256 fingerprint of server.pem (set TELEMETRY_TLS_SERVER_PIN)
set -euo pipefail

OUT="${1:-./telemetry-tls}"
SAN="${2:-DNS:wingman-telemetry}"
DAYS=825
CA_SUBJ="/CN=Sigma Racer Telemetry CA/O=Sigma Tactical Group"
SRV_SUBJ="/CN=wingman-telemetry/O=Sigma Tactical Group"
CLI_SUBJ="/CN=sigma-racer-mechanic/O=Sigma Tactical Group"

mkdir -p "$OUT"
chmod 700 "$OUT"

cat > "$OUT/server.ext" <<EOF
basicConstraints = CA:FALSE
keyUsage = digitalSignature, keyEncipherment
extendedKeyUsage = serverAuth
subjectAltName = $SAN
EOF

cat > "$OUT/client.ext" <<EOF
basicConstraints = CA:FALSE
keyUsage = digitalSignature
extendedKeyUsage = clientAuth
EOF

openssl ecparam -name prime256v1 -genkey -noout -out "$OUT/ca.key"
openssl req -x509 -new -key "$OUT/ca.key" -sha256 -days "$DAYS" -subj "$CA_SUBJ" -out "$OUT/ca.pem"

openssl ecparam -name prime256v1 -genkey -noout -out "$OUT/server.key"
openssl req -new -key "$OUT/server.key" -subj "$SRV_SUBJ" -out "$OUT/server.csr"
openssl x509 -req -in "$OUT/server.csr" -CA "$OUT/ca.pem" -CAkey "$OUT/ca.key" -CAcreateserial \
  -out "$OUT/server.pem" -days "$DAYS" -sha256 -extfile "$OUT/server.ext"

openssl ecparam -name prime256v1 -genkey -noout -out "$OUT/client.key"
openssl req -new -key "$OUT/client.key" -subj "$CLI_SUBJ" -out "$OUT/client.csr"
openssl x509 -req -in "$OUT/client.csr" -CA "$OUT/ca.pem" -CAkey "$OUT/ca.key" -CAcreateserial \
  -out "$OUT/client.pem" -days "$DAYS" -sha256 -extfile "$OUT/client.ext"

openssl x509 -in "$OUT/server.pem" -outform DER | sha256sum | awk '{print $1}' > "$OUT/server.pin"

rm -f "$OUT"/*.csr "$OUT"/*.ext "$OUT"/ca.srl
chmod 600 "$OUT"/*.key
chmod 644 "$OUT"/*.pem "$OUT/server.pin"

echo "Generated telemetry PKI in $OUT"
echo "  Wingman: ca.pem + server.pem + server.key -> /etc/sigma-racer-wingman/telemetry-tls/"
echo "  Mechanic: ca.pem + client.pem + client.key -> ~/.config/sigma-racer-mechanic/tls/"
echo "  Optional pin: TELEMETRY_TLS_SERVER_PIN=$(cat "$OUT/server.pin")"
