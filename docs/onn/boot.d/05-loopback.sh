#!/bin/sh
# BusyBox init brings up no interfaces, loopback included, so anything that binds 127.0.0.1
# fails with "Cannot assign requested address". Measured on the onn on 2026-10-23: lo was DOWN.
set -eu
ip link set lo up 2>/dev/null || chroot /os /usr/bin/ip link set lo up
echo "loopback: $(cat /sys/class/net/lo/operstate)"
