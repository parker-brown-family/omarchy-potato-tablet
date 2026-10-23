#!/bin/sh
# Battery level. MediaTek's battery driver reports capacity -1 until its vendor fuel-gauge daemon
# connects over netlink (measured 2026-10-23: capacity went from -1 to 99 the moment it ran).
# fuelgauged is a 32-bit bionic binary and needs only the 32-bit runtime paths. It needs /vendor,
# /system and /apex, which 20-wifi.sh mounts, so this runs after it. Its chatter goes nowhere:
# /tmp is RAM and it logs every few seconds.
set -eu
if pidof fuelgauged >/dev/null; then echo "battery: fuelgauged already running"; exit 0; fi
test -x /vendor/bin/fuelgauged
cd /tmp
setsid env LD_LIBRARY_PATH=/apex/com.android.runtime/lib/bionic:/system/lib:/vendor/lib /vendor/bin/fuelgauged </dev/null >/dev/null 2>&1 &
sleep 5
echo "battery: capacity $(cat /sys/class/power_supply/battery/capacity)%"
