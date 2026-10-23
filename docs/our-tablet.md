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

- **The desktop.** Hyprland, the Omarchy shell and theme, Foot and the keyboard come up by
  themselves at boot. A boot check waits 60 seconds for the compositor, the keyboard, a
  mapped window, non-black pixels and an answering shell before it calls the boot good.
- **Touch typing.** Parker typed `hello` on the on-screen keyboard and it appeared, confirmed
  in a framebuffer capture, after a fresh reboot with the calibration saved.
- **Software rendering.** The PowerVR device refuses to open, so the compositor draws on the
  CPU with Mesa's llvmpipe (EGL 1.5, OpenGL ES 3.2) and writes to the framebuffer. Refresh is
  capped at 10 Hz by choice. Nobody has measured how responsive that feels.
- **A way back.** A rescue command on the tablet, and a 900-second boot timer, both return it
  to Fastboot.

## What does not, yet

| | State |
|---|---|
| Wi-Fi | no network interface appears |
| Audio | sound cards enumerate; PipeWire and speaker routing are incomplete |
| Battery | status says `Charging`; capacity reads `-1`, which means unavailable |
| GPU acceleration | none: no usable PowerVR driver |
| Suspend | not tested |
| Rotation | not tested; whether the accelerometer is exposed is not yet known |
| Multitouch and gestures | not tested beyond typing |
| Storage of the home folder | the desktop runs as root with a home folder in RAM, so files there vanish on reboot |
| Restoring Android | not demonstrated: the original Android 11 boot partition was overwritten before it was captured |

## From power-on to the desktop

There is no password anywhere on the way:

1. Power on. The bootloader loads our boot image.
2. BusyBox mounts the Arch root. The disk is not encrypted.
3. The desktop starts as root. There is no login screen.
4. The Omarchy shell's idle lock and screensaver are both set to `0` in its config, which
   turns them off.

So the first thing that needs a finger is opening a program. To be confirmed on the live
tablet: whether the bootloader's unlocked-device warning adds a pause, and that the idle lock
really stays off.

## How it is reached

There is no Wi-Fi, so there is no SSH and no remote desktop. The tablet appears on the laptop
as a USB serial device, and every command and screen capture goes over that cable. A capture
is the raw framebuffer (1200 × 1920, 4,864-byte rows), read back and decoded on the laptop.

## Not published yet

The port's own code (the build scripts, the Hyprland and Aquamarine patches, the boot image
tooling) still contains exact-device paths and private references, and needs a publishing pass
before it can live here. Vendor images, serial numbers and raw logs will never be published.
