#!/usr/bin/env bash
# Bring up a virtual CAN interface for bench-testing the cluster's SocketCAN
# source (CLUSTER_TELEMETRY_SOURCE=can). Linux only; needs the vcan kernel
# module and iproute2.
#
#   sudo scripts/vcan-up.sh [iface]      # default: vcan0
#
# Then, in another shell, feed it frames — e.g. replay the baked sample:
#
#   canplayer -I testdata/sample-ride.log vcan0=can1   # remap can1 -> vcan0
#   # or a single frame:
#   cansend vcan0 0A0#B004525C00000000
#
# and run the cluster against it:
#
#   CLUSTER_TELEMETRY_SOURCE=can CLUSTER_CAN_IFACE=vcan0 \
#     cargo run --features can-socket
set -euo pipefail

IFACE="${1:-vcan0}"

if [[ $EUID -ne 0 ]]; then
    echo "error: must run as root (sudo) to configure network interfaces" >&2
    exit 1
fi

modprobe vcan

if ip link show "$IFACE" >/dev/null 2>&1; then
    echo "$IFACE already exists"
else
    ip link add dev "$IFACE" type vcan
    echo "created $IFACE"
fi

ip link set up "$IFACE"
echo "$IFACE is up"
