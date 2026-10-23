#!/bin/sh
# Run every hook in the Arch root's /etc/onn-omarchy/boot.d, in name order, after the desktop has
# been launched. Hooks live on the persistent disk, so changing what starts at boot needs no new
# boot image. Each hook gets 180 seconds; one failing does not stop the next.
for hook in /os/etc/onn-omarchy/boot.d/*.sh; do
    [ -f "$hook" ] || continue
    echo "== $hook at $(cut -d' ' -f1 /proc/uptime)s"
    timeout 180 /bin/sh "$hook"
    echo "== exit $? at $(cut -d' ' -f1 /proc/uptime)s"
done
