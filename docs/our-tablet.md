# Our tablet: the onn 10.1" Tablet Pro

A Walmart onn 10.1" Tablet Pro (model 100003562, board `mid1035_mq`, about 2020) that shipped
with Android 11. Since 2026-10-22 it boots Omarchy natively: Android is gone and nothing runs
underneath. This page is the state as measured on 2026-10-22, and nothing on it is claimed
beyond what was measured.

| | |
|---|---|
| SoC | MediaTek MT8768 (8× Cortex-A53) |
| GPU | PowerVR GE8320. No open driver, so nothing here uses it |
| Screen | 1200 × 1920 |
| RAM, storage | 3 GB, 32 GB eMMC |
| Kernel | the tablet's own Android recovery kernel, Linux 4.19.127, and its own device tree |

## What runs

| Component | Version |
|---|---|
| Omarchy | 4.0.6-1 |
| Hyprland | 0.56.2, with local framebuffer patches |
| Quickshell (the Omarchy shell) | 0.3.2 |
| Foot | 1.28.0 |
| On-screen keyboard | wvkbd 0.21 |
| Userspace | Arch Linux ARM, an 8 GiB root on the old Android userdata partition |

BusyBox is PID 1. It mounts the Arch root and starts the desktop through `chroot`. systemd is
not used, because systemd 262 needs Linux 5.10 or newer and this kernel is 4.19.

## What works

Measured on 2026-10-23 after cold reboots, with nobody touching the tablet.

| | State | How |
|---|---|---|
| Desktop: Hyprland, the Omarchy shell, Foot, a keyboard | up at boot; a 60-second check confirms it | the boot image's own init |
| Wi-Fi | associated about 20 s after power-on, same address every boot | [20-wifi.sh](onn/boot.d/20-wifi.sh): the stock vendor daemons, run without Android |
| SSH | up 4 s after power-on, key login only | [10-sshd.sh](onn/boot.d/10-sshd.sh) |
| Packages | `pacman` installs over Wi-Fi | `DisableSandbox` in `pacman.conf` (see gotchas) |
| Battery level | reads a real percentage | [30-battery.sh](onn/boot.d/30-battery.sh): the vendor fuel-gauge daemon |
| Tablet layer | Ragtop, in tablet mode, with its touch menu and lock-screen keyboard | installed with `omarchy plugin add`, setup driven by the diagnostic kit's synthetic taps |
| Touch typing | works on both keyboards | calibrated from Parker's physical taps |
| A web app, by touch | Chromium 153 runs Orthodox Lampada in an app window; tapping, typing on the on-screen keyboard and scrolling all reach it | [onn-webapp](onn/bin/onn-webapp) and its [flags](onn/bin/chromium-flags.conf); walked through by the kit's [journey](../kit/README.md#walk-through-an-app), all 36 checks passing |
| Loopback | up at boot | [05-loopback.sh](onn/boot.d/05-loopback.sh): BusyBox init leaves `lo` down |
| Software rendering | llvmpipe, refresh capped at 10 Hz | the PowerVR GPU has no open driver |
| A persistent home | plugins and settings survive reboot | the boot image binds `/var/lib/onn-omarchy/home` over the desktop's home |
| A way back | a rescue command and a 15-minute boot timer return it to Fastboot | |

Everything that starts at boot lives in `/etc/onn-omarchy/boot.d` on the Arch root, and
[run-boot-hooks.sh](onn/run-boot-hooks.sh) runs it. Changing what starts no longer needs a new boot image.

## What does not, yet

| | State |
|---|---|
| Speakers | the amplifier controls switch on and the stream plays; whether sound comes out has not been heard |
| Rotation | the accelerometer reads through `/sys/class/sensor/m_acc_misc` (not IIO), but Ragtop's rotation would overwrite the touch calibration, so it stays locked |
| GPU acceleration | none: no usable driver for the PowerVR GE8320 |
| Suspend | not tested |
| Two keyboards | wvkbd and Ragtop's keyboard both run; Ragtop's handle moved wvkbd up 36 px, which the diagnostic kit caught |
| Restoring Android | not demonstrated: the original boot partition was overwritten before it was captured |

## From power-on to the desktop

There is no password anywhere on the way:

1. Power on. The bootloader loads our boot image.
2. BusyBox mounts the Arch root. The disk is not encrypted.
3. The desktop starts as root. There is no login screen.
4. The shell's idle lock and screensaver are both `0`, which turns them off.

## How it is reached

SSH over Wi-Fi, as root with a key. The USB serial gadget still works when the network does not.
Screenshots come from the framebuffer: 1200 × 1920, 4,864-byte rows, under a second over Wi-Fi.

## Not published yet

The port's own code (the build scripts, the Hyprland and Aquamarine patches, the boot image
tooling) still contains exact-device paths and private references, and needs a publishing pass
before it can live here. Vendor images, serial numbers and raw logs will never be published.
