# How we did it

The order the onn tablet went in, on 2026-10-21 and 2026-10-22. Each step names what it proved,
because the next step depended on it. A step we would do differently says so.

## 1. Find out exactly what the tablet is

Read the truth off the device, not the spec sheet. With USB debugging on:

```sh
adb shell "uname -r; getprop ro.hardware; getprop ro.build.version.release; getprop ro.boot.verifiedbootstate; getprop sys.oem_unlock_allowed"
```

Ours came back Android 11, MT8768, kernel 4.19, OEM unlock allowed. A sample kernel config from
a public device tree for the exact board matched ours byte for byte, which told us the kernel's
features before we touched anything.

## 2. Look for people who went first

We ran a research pass for native ports, kernels and Android-hosted routes before choosing one.
Two ports shaped ours: a Xiaomi Pad 9 Pro port that boots Omarchy on the stock Android kernel,
whose staged boot and return-to-bootloader method we borrowed, and mtkclient for talking to the
MediaTek chip. [Other people's work](ecosystem.md) has the full list.

## 3. Back everything up, then unlock

We captured recovery, DTBO, vbmeta, the eMMC boot area and both partition tables. **We did not
capture the stock boot partition before the first write, and that is the one thing we would
change.** Without it, a full return to Android has not been shown.

Unlocking the bootloader erases userdata. Our receipts record that it was unlocked, but not the
command used.

## 4. Boot something tiny first

The first boot image used the tablet's own recovery kernel and device tree with a BusyBox
initramfs: no desktop, just a shell over a USB serial gadget and a timer that returns to
Fastboot. Proving that the way back works came before anything that could fail to boot.

## 5. Put Arch Linux ARM on userdata

An 8 GiB Arch Linux ARM root with Omarchy's packages, written to the old userdata partition in
36 chunks, then mounted and entered with `chroot`. Bash and the package database worked from the
serial shell.

## 6. Get pixels on the screen

The stock path failed twice. systemd would not run on a 4.19 kernel, so BusyBox stayed as PID 1.
The PowerVR graphics device refused to open. The fix was local patches: Aquamarine got a
shared-memory allocator and a framebuffer presenter, Hyprland got a surfaceless software path
through Mesa's llvmpipe, and a process-local libudev-zero let Hyprland find its devices. Three
bugs stood between a running compositor and a visible desktop: a black screen from a blank
buffer replacing the rendered one, a readback that needed the read framebuffer bound explicitly,
and text upside down from one Y flip too many.

## 7. Make touch land where you tap

The Himax touchscreen reports multitouch axes without the legacy single-touch ones, so it was
not recognised as a touchscreen until libudev-zero was patched. Then calibration: the screen is
rotated a quarter turn from the digitizer. **Tapping Q could not tell a flip from a quarter turn,
because both put the tap inside the Q key.** Typing a word that crosses rows and columns could,
and `hello` settled it: transform 3, matrix `[0,1,0,-1,0,1]`.

## 8. Let the boot decide whether it worked

Every boot now checks itself before cancelling its return-to-Fastboot timer: compositor running,
keyboard up, a window mapped, pixels that are not black, the Omarchy shell answering.

## 9. Get it on the network, and keep everything across reboots

Wi-Fi came up by hand at the end of night one, by running the stock MediaTek connectivity daemons
without Android. Day two made it permanent. The boot image gained one line: run every script in
`/etc/onn-omarchy/boot.d` on the Arch root. After that, what starts at boot is a file on disk, and
no change needs a new boot image. Three scripts went there: [SSH](onn/boot.d/10-sshd.sh),
[Wi-Fi](onn/boot.d/20-wifi.sh) with a pinned MAC address, and the [battery gauge](onn/boot.d/30-battery.sh).
The image also binds a persistent home over the desktop's RAM home, so plugins survive. With a
network, `pacman` installed Ragtop's dependencies and Gliff, and the kit's own synthetic taps
clicked through Ragtop's setup.

## Tools we leaned on

| Tool | For |
|---|---|
| adb, fastboot | reading the device, flashing the boot partition |
| mtkclient | MediaTek transport |
| a USB serial gadget | the only link once Android was gone |
| framebuffer readback | the tablet's own screenshot, decoded on the laptop |
| research-delight | the sourced research passes behind every choice |
| [the diagnostic kit](../kit/README.md) | synthetic taps and checked screenshots, over serial or SSH |
