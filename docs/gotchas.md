# Gotchas

Each one cost somebody time. The source is on every line: ours means it happened on the onn
tablet, and a link means someone else found it.

## Android tablets, replacing Android

- **Capture the stock boot partition before the first write.** We didn't, and without it a full
  Android restore has not been shown. Recovery, DTBO, vbmeta, the eMMC boot area and both
  partition tables were saved, and still don't add up to a way back. *(ours)*
- **Unlocking the bootloader erases userdata**, and with it every installed Android app.
  *(ours)*
- **Don't relock the bootloader** on a device running your own boot image. *(ours)*
- **Learn the button combination early.** On the onn: Power + Volume Up opens boot selection,
  Volume Up moves the highlight, Volume Down selects Fastboot. There are also Chinese
  factory-test menus that are a different mode. *(ours)*
- **An old Android kernel won't run modern systemd.** systemd 262 needs Linux 5.10 or newer;
  this tablet has 4.19. BusyBox as PID 1 works.
  ([systemd README](https://github.com/systemd/systemd/blob/v262/README))
- **PowerVR GPUs have no open driver.** The DRM device refused to open (`ENODEV`), so the desktop
  renders on the CPU. *(ours)*
- **A healthy compositor can still draw a black screen.** A mapped window and running processes
  are not proof of pixels. Read the framebuffer back. *(ours)*
- **Bind the read framebuffer explicitly before CPU readback**, or you read the wrong buffer.
  *(ours)*
- **A touchscreen with only multitouch axes may not be detected as one.** The Himax panel lacks
  legacy `ABS_X`/`ABS_Y`, and libudev-zero needed a patch to classify it. *(ours)*
- **Calibrate with a word, not a key.** One tap on Q landed inside Q under both a flip and a
  quarter turn. `hello`, which crosses rows and columns, told them apart. *(ours)*
- **Never read `/sys/power/wakeup_count` with a blocking `cat` or `dd`.** It hung our serial
  shell. *(ours)*
- **An empty `/etc/machine-id` stops autostart**, and a health check over Quickshell IPC needs
  `WAYLAND_DISPLAY` set. *(ours)*
- **Rendering cost scales with pixels.** 1200 × 1920 is 2.3 million pixels for eight Cortex-A53s
  to paint in software. We capped refresh at 10 Hz. *(ours)*

## Omarchy without systemd, on an Android kernel

All measured on the onn on 2026-10-23, Linux 4.19 with BusyBox as PID 1 and Arch in a chroot.

- **Wi-Fi works without Android, through the stock vendor daemons.** Mount the vendor and system
  partitions read-only, mount the APEX runtime, load `wmt_drv.ko`, run `wmt_loader` and
  `wmt_launcher` under the bionic runtime with a small `LD_PRELOAD` that answers property lookups,
  load the Wi-Fi modules, `printf 1 > /dev/wmtWifi`, then `wpa_supplicant` and `udhcpc`.
  [The script](onn/boot.d/20-wifi.sh). *(ours)*
- **The Wi-Fi MAC changes every boot** without Android's NVRAM service, so the router hands out a
  new address every time. Pin one before associating. *(ours)*
- **Vendor daemons can be 32-bit.** `fuelgauged` wanted `/apex/com.android.runtime/lib/bionic`,
  `/system/lib` and `/vendor/lib`, not the `lib64` paths, and refused a 64-bit `LD_PRELOAD`.
  *(ours)*
- **Battery capacity reads -1 until the vendor fuel-gauge daemon runs.** Started, it went straight to
  99. [The script](onn/boot.d/30-battery.sh). *(ours; cause found in a
  [sibling 4.19 kernel](https://github.com/geevarghesest/android_kernel_mt8768t/blob/main/drivers/power/supply/mtk_battery.c))*
- **pacman 7 fails twice**: Landlock is not in a 4.19 kernel, and its sandbox user may be missing.
  `DisableSandbox` under `[options]` in `/etc/pacman.conf` fixes both. *(ours)*
- **`systemd-sysusers` fails on 4.19** ("Protocol driver not attached"), so system users a package
  expects may never be created. *(ours)*
- **A mount made after the chroot's bind does not appear inside it.** `/run` and `/apex` were bound
  into the Arch root at boot, so a later mount under either was invisible to the desktop: the
  persistent home, and the APEX runtime that `/system/bin/linker64` points into. Bind again inside
  the chroot's path. *(ours)*
- **Omarchy restarts its shell through `systemd-cat`**, which needs the journal. A
  [stand-in](onn/bin/systemd-cat) in `/usr/local/bin` that writes to `/tmp` makes Omarchy's own restart work.
  *(ours)*
- **Notifications and Ragtop's setup need the session bus address**, which lives only in Hyprland's
  environment when the desktop runs under `dbus-run-session`. Read it from `/proc/<Hyprland>/environ`.
  [desktop-env.sh](onn/bin/desktop-env.sh). *(ours)*
- **Ragtop's keyboard needs `pkg-config`** (the `pkgconf` package) to generate its Wayland bindings;
  without it the keyboard draws no letters. *(ours)*
- **`reboot` inside the chroot is systemd's and does nothing.** Use BusyBox's `reboot -f` from the
  outer environment. *(ours)*
- **Ragtop rotates touch and screen with one transform**, which overwrites a touch calibration that
  needs its own (the onn's is transform 3). Keep rotation locked until touch can carry an offset.
  *(ours)*
- **The accelerometer is not IIO on MediaTek 4.19.** Enable it with
  `echo 0,0,66667000,0 > /sys/class/sensor/m_acc_misc/accbatch` and `echo 1 > .../accactive`, then read
  `/sys/bus/platform/drivers/gsensor/sensordata` (hex x y z, about 9807 for one g). *(ours; path from the
  [sibling kernel](https://github.com/geevarghesest/android_kernel_mt8768t/blob/main/drivers/misc/mediatek/sensors-1.0/accelerometer/accel.c))*

## Android tablets, keeping Android

- **omarchy-android's installer wants Android 12 (API 31) or newer**, and its software-graphics
  option still selects VirGL. A Galaxy Tab S9 Ultra report shows VirGL failing to start.
  ([installer](https://github.com/BlackFireAlex/omarchy-android),
  [report](https://github.com/BlackFireAlex/omarchy-android/issues/4))

## x86 tablets that took the Omarchy ISO

- **The disk passphrase at boot takes no touch.** Omarchy encrypts the disk, and the boot splash
  has no on-screen keyboard. Keep a USB keyboard for cold boots, or remove the prompt (TPM2,
  FIDO2, a keyfile, or no encryption).
  ([Surface Go runbook](https://github.com/kermes/surface-go-omarchy-setup/blob/main/07-gotchas.md))
- **TPM2 unlock plus autologin means no authentication at all**, and a binding that includes
  PCR 11 breaks on every kernel update.
  ([same runbook](https://github.com/kermes/surface-go-omarchy-setup/blob/main/07-gotchas.md))
- **Omarchy's screensaver ignores touch**: only a key press inside its own terminal dismisses it.
  ([omacom/omarchy#7762](https://github.com/omacom/omarchy/issues/7762))
- **Omarchy's menu closes when you tap an on-screen key**, because it takes exclusive keyboard
  focus. Ragtop works around it with clones of the overlays.
  ([omacom/omarchy#9029](https://github.com/omacom/omarchy/issues/9029),
  [Ragtop](https://github.com/sclemance/ragtop))
- **Surface Pro and Book touchscreens need linux-surface** (or an IPTS DKMS module). Surface Go's
  ELAN screen works on the stock kernel.
  ([omarchy-surface-touch](https://github.com/javon27/omarchy-surface-touch),
  [surface-touch-omarchy](https://github.com/ImNickRichard/surface-touch-omarchy))
- **squeekboard never auto-shows unless the GNOME accessibility key is on**:
  `gsettings set org.gnome.desktop.a11y.applications screen-keyboard-enabled true`.
  ([Surface Go runbook](https://github.com/kermes/surface-go-omarchy-setup/blob/main/04-onscreen-keyboard.md))
- **hyprgrass stops loading after every Hyprland update** until `hyprpm update` runs again.
  ([Surface Go runbook](https://github.com/kermes/surface-go-omarchy-setup))
- **The Surface Go's cameras hard-lock the machine**: blacklist the IPU3 modules.
  ([Surface Go runbook](https://github.com/kermes/surface-go-omarchy-setup))

## Commands worth keeping

```sh
# x86: what is this tablet, and does the kernel see touch and an accelerometer?
cat /sys/class/dmi/id/sys_vendor /sys/class/dmi/id/product_name; uname -r; omarchy version; hyprctl devices | grep -i touch; ls /sys/bus/iio/devices
```

```sh
# Android, before anything is changed
adb shell "uname -r; getprop ro.hardware; getprop ro.build.version.release; getprop ro.boot.verifiedbootstate; getprop sys.oem_unlock_allowed"
```

```sh
# Ragtop, the tablet layer, on Omarchy 4
omarchy pkg add iio-sensor-proxy python-pywayland python-gobject
omarchy plugin add https://github.com/sclemance/ragtop.git --enable
```
