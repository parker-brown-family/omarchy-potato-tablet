#!/bin/sh
# Bring up Wi-Fi on the onn without Android, the path measured on 2026-10-22:
# stock vendor and system partitions read-only, the APEX runtime for bionic binaries, the WMT
# connectivity modules, the stock wmt_loader and wmt_launcher with a property shim, power on,
# then wpa_supplicant and udhcpc from the Arch root. Every step skips itself if already done.
set -eu
L=/os/etc/onn-omarchy
RT=/apex/com.android.runtime/lib64/bionic:/system/lib64:/vendor/lib64

dm_node() {   # the /dev/dm-N node of a named device-mapper table, or nothing
    for d in /sys/block/dm-*; do
        [ -f "$d/dm/name" ] || continue
        if [ "$(cat "$d/dm/name")" = "$1" ]; then echo "/dev/${d##*/}"; return; fi
    done
}

# 1 · vendor and system, read-only, from the super partition (offsets read with lpdump on this unit)
grep -q '^PARTNAME=super$' /sys/class/block/mmcblk0p35/uevent
test "$(cat /sys/class/block/mmcblk0p35/size)" = 8388608
[ -n "$(dm_node onn-vendor)" ] || chroot /os /usr/bin/dmsetup --noudevsync --noudevrules create onn-vendor --readonly --table '0 549456 linear /dev/mmcblk0p35 3190784'
[ -n "$(dm_node onn-system)" ] || chroot /os /usr/bin/dmsetup --noudevsync --noudevrules create onn-system --readonly --table '0 2394040 linear /dev/mmcblk0p35 3741696'
mdev -s
mkdir -p /vendor /os/vendor /system-root /system /os/system /apex/com.android.runtime /os/apex
grep -q ' /vendor ' /proc/mounts || mount -t ext4 -o ro,noload "$(dm_node onn-vendor)" /vendor
grep -q ' /os/vendor ' /proc/mounts || mount --bind /vendor /os/vendor
grep -q ' /system-root ' /proc/mounts || mount -t ext4 -o ro,noload "$(dm_node onn-system)" /system-root
if ! grep -q ' /system ' /proc/mounts; then
    if [ -d /system-root/system/bin ]; then mount --bind /system-root/system /system; else mount --bind /system-root /system; fi
fi
grep -q ' /os/system ' /proc/mounts || mount --bind /system /os/system
echo "wifi: vendor and system mounted"

# 2 · the bionic runtime the vendor daemons need
[ -f /tmp/onn-runtime-apex.img ] || unzip -p /system/apex/com.android.runtime.apex apex_payload.img > /tmp/onn-runtime-apex.img
grep -q ' /apex/com.android.runtime ' /proc/mounts || mount -t ext4 -o loop,ro,noload /tmp/onn-runtime-apex.img /apex/com.android.runtime
grep -q ' /os/apex ' /proc/mounts || mount --bind /apex /os/apex
mkdir -p /lib
[ -e /lib/firmware ] || ln -s /vendor/firmware /lib/firmware
[ -e /etc/firmware ] || ln -s /vendor/firmware /etc/firmware

# 3 · the WMT connectivity stack
grep -q '^wmt_drv ' /proc/modules || insmod /vendor/lib/modules/wmt_drv.ko
mdev -s
if [ ! -c /dev/stpwmt ]; then
    timeout 15 env LD_LIBRARY_PATH="$RT" /vendor/bin/wmt_loader || echo "wifi: wmt_loader exit $?"
    mdev -s
fi
test -c /dev/stpwmt
if ! pidof wmt_launcher >/dev/null; then
    setsid env LD_PRELOAD="$L/lib/wmt-property-bridge.so" LD_LIBRARY_PATH="$RT" \
        /vendor/bin/wmt_launcher -p /vendor/firmware/ -o 1 </dev/null >/tmp/wmt-launcher.log 2>&1 &
    sleep 6
fi
grep -q '^wmt_chrdev_wifi ' /proc/modules || timeout 20 insmod /vendor/lib/modules/wmt_chrdev_wifi.ko
grep -q '^wlan_drv_gen4m ' /proc/modules || timeout 20 insmod /vendor/lib/modules/wlan_drv_gen4m.ko
mdev -s
echo "wifi: WMT up"

# 4 · power the radio and wait for wlan0
if [ ! -d /sys/class/net/wlan0 ]; then
    test -c /dev/wmtWifi
    timeout 25 sh -c 'printf 1 > /dev/wmtWifi'
fi
i=0
while [ ! -d /sys/class/net/wlan0 ] && [ "$i" -lt 20 ]; do sleep 1; i=$((i + 1)); done
test -d /sys/class/net/wlan0
# Without Android's NVRAM service the driver invents a new MAC address every boot, so the router
# hands out a new address every boot. Pin one, kept in $L/wifi-mac on the persistent disk.
if [ -s "$L/wifi-mac" ] && [ "$(cat /sys/class/net/wlan0/address)" != "$(cat "$L/wifi-mac")" ]; then
    ip link set wlan0 down
    ip link set wlan0 address "$(cat "$L/wifi-mac")"
fi
ip link set wlan0 up
echo "wifi: wlan0 present, address $(cat /sys/class/net/wlan0/address)"

# 5 · associate, then keep a DHCP lease (udhcpc stays running to renew it)
mkdir -p /run/wpa_supplicant
pidof wpa_supplicant >/dev/null || chroot /os /usr/bin/wpa_supplicant -B -i wlan0 -D nl80211 -c /etc/onn-omarchy/wpa.conf -f /tmp/wpa-native.log
i=0
until chroot /os /usr/bin/wpa_cli -i wlan0 status 2>/dev/null | grep -q '^wpa_state=COMPLETED$'; do
    i=$((i + 1))
    if [ "$i" -gt 40 ]; then echo "wifi: no association after 40 s"; tail -20 /tmp/wpa-native.log; exit 1; fi
    sleep 1
done
echo "wifi: associated"
pidof udhcpc >/dev/null || udhcpc -i wlan0 -s "$L/udhcpc-onn.sh" -b -t 10 -T 3
sleep 3
ip -4 addr show dev wlan0 | grep inet || echo "wifi: no address yet"
