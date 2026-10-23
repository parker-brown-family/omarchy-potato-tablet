#!/bin/sh
# Start the Arch root's OpenSSH server, key login only (see /etc/ssh/sshd_config.d/10-onn.conf).
# Runs from the boot image's hook loop in the BusyBox environment, with the Arch root at /os.
set -eu
mkdir -p /run/sshd
if pidof sshd >/dev/null; then
    echo "sshd already running: $(pidof sshd)"
    exit 0
fi
setsid chroot /os /usr/bin/sshd -E /tmp/sshd.log </dev/null >/dev/null 2>&1 &
sleep 2
pidof sshd >/dev/null && echo "sshd started: $(pidof sshd)"
