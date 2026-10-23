# Other people's work

Everything here was read at its source on 2026-10-23. A value marked _unconfirmed_ is one
nobody could check, and it is written as a flag rather than a guess.

Generated from `data/ecosystem.json` by `scripts/build-ecosystem.py`. Edit the data, not this page.

## Tablet layers: install on top of Omarchy 4

### Ragtop

- **What it gives a tablet:** Tablet mode from the kernel switch, auto-rotation with a lock, an on-screen keyboard drawn by the Omarchy shell in the theme's colours (phone, 60%, 75%, 80% boards, one-shot Ctrl/Super, accents by long press, swipe), touch window arranging (tiling mode, preview), optional typing into Omarchy's menu/overlays and an optional lock-screen keyboard. ([source](https://github.com/sclemance/ragtop))
- **How it goes on:** In the official plugin marketplace (sclemance.ragtop). `omarchy pkg add iio-sensor-proxy python-pywayland` then `omarchy plugin add https://github.com/sclemance/ragtop.git --enable`; setup opens in the shell by itself. ([source](https://github.com/sclemance/ragtop))
- **Tested on:** Lenovo 300w Yoga Gen 4 (convertible) and Surface Book 2 (detachable; touch itself needs linux-surface on that machine). Beta: 'only been tried on two machines'. ([source](https://github.com/sclemance/ragtop))
- **With no keyboard at all:** Supported but untested: on a slate with no tablet-mode switch, set Tablet Mode > Always On once. The README asks for slate reports. ([source](https://github.com/sclemance/ragtop))
- **Who made it:** Stan Clemance (github.com/sclemance). Credits Gimbal for the overlay focus fix. ([source](https://github.com/sclemance))
- **Licence:** MIT ([source](https://github.com/sclemance/ragtop))
- **When:** Created 2026-09-20, last push 2026-10-22. ([source](https://github.com/sclemance/ragtop))

### Omarchy Tablet (launcher)

- **What it gives a tablet:** A touch launcher with favourites and search, a window switcher (swipe up), Single-app layout, squeekboard opening on text fields, dictation buttons, and Tablet / Desktop / Automatic modes keyed on whether a physical keyboard is present. ([source](https://github.com/varlet99/omarchy-tablet))
- **How it goes on:** In the marketplace as surface.tablet. Needs Python 3.11+ and python-gobject; replaces the whole bar with one top row. ([source](https://github.com/varlet99/omarchy-tablet))
- **Tested on:** Surface Pro 4, Omarchy 4.0.4-1, Hyprland 0.56.2, linux-surface 6.19.8. Others 'not yet validated'. ([source](https://github.com/varlet99/omarchy-tablet))
- **With no keyboard at all:** Yes: it is the only one built around a launcher, which is what a slate with no keyboard needs to open apps. ([source](https://github.com/varlet99/omarchy-tablet))
- **Who made it:** Charles Rivest (github.com/varlet99). ([source](https://github.com/varlet99))
- **Licence:** MIT ([source](https://github.com/varlet99/omarchy-tablet))
- **When:** Created 2026-09-28, last push 2026-10-19. ([source](https://github.com/varlet99/omarchy-tablet))

### omarchy-surface-touch

- **What it gives a tablet:** Ten independent components: patched wvkbd keyboard, floating virtual trackpad, two-finger right-click, accelerometer auto-rotate, touch-dismissable screensaver, PIN lock screen with keypad, Type Cover touchpad rebind, keyboard only while the cover is detached, and an iptsd crash override. ([source](https://github.com/javon27/omarchy-surface-touch))
- **How it goes on:** Clone and run each component's installer; several need root (uinput injector, PAM PIN, udev). Not a marketplace plugin. ([source](https://github.com/javon27/omarchy-surface-touch))
- **Tested on:** Surface Pro 7+ on Omarchy 4.x with linux-surface. Device values are detected at runtime; ELAN-digitizer Surfaces (Go) skip the kernel part. ([source](https://github.com/javon27/omarchy-surface-touch))
- **With no keyboard at all:** Yes. Built to use the tablet with the keyboard detached, including unlocking by PIN. ([source](https://github.com/javon27/omarchy-surface-touch))
- **Who made it:** javon27 (github.com/javon27); Reddit post 'Touch-enabled my Surface Pro 7+ for Omarchy'. kermes maintains a fork used by the Surface Go runbook. ([source](https://github.com/javon27))
- **Licence:** MIT ([source](https://github.com/javon27/omarchy-surface-touch))
- **When:** Created 2026-08-25, last push 2026-10-07, 15 stars: the oldest of the set. ([source](https://github.com/javon27/omarchy-surface-touch))

### omarchy-osk

- **What it gives a tablet:** A Hyprland compositor plugin that turns touch into pointer input (tap, drag, long-press right-click, two-finger scroll with fling, pinch to Ctrl+wheel) plus an Omarchy-shell keyboard that injects real keycodes, and an edge swipe through vendored hyprgrass. ([source](https://github.com/ekollof/omarchy-osk))
- **How it goes on:** `./install.sh` builds the compositor plugins; `omarchy plugin add` installs only the shell UI. Compositor plugins must match the Hyprland version. ([source](https://github.com/ekollof/omarchy-osk))
- **Tested on:** Screenshot shows a tablet in portrait; no model list in the README. The author's upstream PR fixes GPD Pocket 4 touch orientation. _(unconfirmed)_
- **With no keyboard at all:** Yes by design: 'turns a touchscreen laptop/tablet into a fully usable machine without a physical keyboard'. Rotation is separate. ([source](https://github.com/ekollof/omarchy-osk))
- **Who made it:** Emiel 'Andrath' Kollof (github.com/ekollof, x.com/Andrath1975). Also reported the upstream bug where Omarchy's menu blocks on-screen keyboard input (omacom/omarchy issue 9029). ([source](https://github.com/ekollof))
- **Licence:** GitHub reports NOASSERTION; read the repository before reusing code. _(unconfirmed)_
- **When:** Created 2026-08-31, last push 2026-10-14. ([source](https://github.com/ekollof/omarchy-osk))

### Omarchy dotfiles for tablets

- **What it gives a tablet:** Dotfiles for detachables: an Omarchy-style keyboard as the input method (pops on text fields while detached), Stay Awake with the cover closed, lock-screen PIN pad, bar pinned in tablet mode, touch tray, hyprgrass move/resize by long-press, rotation with touch and pen following, power-saver on battery. ([source](https://github.com/yeutterg/omarchy-tablet))
- **How it goes on:** install.sh in the repo; the keyboard, lock PIN and rotation are separate Omarchy plugins (lock-pin and display-rotation are in the marketplace). ([source](https://github.com/yeutterg/omarchy-tablet))
- **Tested on:** Dell Latitude 7350 Detachable, with a separate hardware repo for that model. ([source](https://github.com/yeutterg/omarchy-tablet))
- **With no keyboard at all:** Yes for a detachable with the cover off. Trade-off stated: most Electron apps will not pop the keyboard. ([source](https://github.com/yeutterg/omarchy-tablet))
- **Who made it:** Greg Yeutter (github.com/yeutterg, gregyeutter.com). ([source](https://github.com/yeutterg))
- **Licence:** No licence file reported by GitHub. _(unconfirmed)_
- **When:** Created 2026-10-05, last push 2026-10-19. ([source](https://github.com/yeutterg/omarchy-tablet))

### Omarchy Tablet (convertibles)

- **What it gives a tablet:** Tablet-switch detection with a manual toggle, a large keyboard with accents and English/Spanish suggestions, touch controls for brightness, volume, workspaces and closing windows, media controls, rotation of display + touch + pen, and a Zen reading mode with grayscale/warm filters. ([source](https://github.com/uripg/omarchy-tablet))
- **How it goes on:** `omarchy plugin add https://github.com/uripg/omarchy-tablet --yes`, then scripts/setup builds the keyboard and scripts/doctor checks it. ([source](https://github.com/uripg/omarchy-tablet))
- **Tested on:** A Lenovo Yoga on Hyprland 0.56.2. 'Early release.' ([source](https://github.com/uripg/omarchy-tablet))
- **With no keyboard at all:** Written for convertibles; manual mode works without a switch, slate use not reported. _(unconfirmed)_
- **Who made it:** Oriol Pujol Garcia (github.com/uripg). ([source](https://github.com/uripg))
- **Licence:** GPL-3.0 ([source](https://github.com/uripg/omarchy-tablet))
- **When:** Created and last pushed 2026-10-02. ([source](https://github.com/uripg/omarchy-tablet))

### Tablet Mode for Omarchy

- **What it gives a tablet:** Fold-back detection, rotation from the accelerometer with touch and pen following, the keyboard underneath switched off, and a rotation lock in the bar. ([source](https://github.com/andreiyurik/omarchy-tablet-mode))
- **How it goes on:** In the marketplace as andreiyurik.tablet-mode. ([source](https://github.com/andreiyurik/omarchy-tablet-mode))
- **Tested on:** ThinkPad X1 Yoga Gen 6; CI tests on GitHub Actions. ([source](https://github.com/andreiyurik/omarchy-tablet-mode))
- **With no keyboard at all:** Written for 360-degree convertibles; not a slate tool. _(unconfirmed)_
- **Who made it:** Andrei Yurik (github.com/andreiyurik). ([source](https://github.com/andreiyurik))
- **Licence:** MIT ([source](https://github.com/andreiyurik/omarchy-tablet-mode))
- **When:** Created 2026-09-09, last push 2026-10-15. ([source](https://github.com/andreiyurik/omarchy-tablet-mode))

### Omarchy Touch Mode

- **What it gives a tablet:** A floating keyboard over the native virtual-keyboard protocol, a reversible tablet mode that journals the desktop before changing it, and rotation from iio-sensor-proxy over D-Bus. ([source](https://github.com/alertxsto/omarchy-touch-mode))
- **How it goes on:** Install build dependencies with pacman yourself, then add the plugin; it never runs sudo or edits Hyprland config. ([source](https://github.com/alertxsto/omarchy-touch-mode))
- **Tested on:** Laptops with one internal display and one touchscreen; no model named. _(unconfirmed)_
- **With no keyboard at all:** No hinge detection: tablet mode is entered by hand, which suits a slate, but no slate is reported. _(unconfirmed)_
- **Who made it:** yKid (github.com/alertxsto). ([source](https://github.com/alertxsto))
- **Licence:** GitHub reports NOASSERTION. _(unconfirmed)_
- **When:** Created and last pushed 2026-10-04. ([source](https://github.com/alertxsto/omarchy-touch-mode))

### Gimbal

- **What it gives a tablet:** Rotation of display, touch and pen, a keyboard laid out like the machine's own, two thumb knobs for gestures, a lock-screen keypad, typing into Omarchy's menu. Nothing runs while it is a laptop. ([source](https://github.com/mechanicsunlocked/gimbal))
- **How it goes on:** `omarchy plugin add https://github.com/mechanicsunlocked/gimbal.git --enable --yes` then its install.sh; no root. ([source](https://github.com/mechanicsunlocked/gimbal))
- **Tested on:** Framework Laptop 12 only (stated requirement). ([source](https://github.com/mechanicsunlocked/gimbal))
- **With no keyboard at all:** Framework 12 convertible only. _(unconfirmed)_
- **Who made it:** mechanicsunlocked (github.com/mechanicsunlocked). Ragtop credits its overlay focus fix. ([source](https://github.com/mechanicsunlocked))
- **Licence:** MIT ([source](https://github.com/mechanicsunlocked/gimbal))
- **When:** Created 2026-08-18, the first of these tablet plugins; last push 2026-09-02. ([source](https://github.com/mechanicsunlocked/gimbal))

## Hardware runbooks: x86 tablets

### Surface Go runbook

- **What it gives a tablet:** A runbook, not a package: touchscreen, 3-finger and edge gestures (hyprgrass), auto-rotation, squeekboard appearing on text-field focus, keyboard gating when detached or folded, touch PIN unlock, fast Limine boot, stylus, suspend. Says which items were observed working and which were only installed. ([source](https://github.com/kermes/surface-go-omarchy-setup))
- **How it goes on:** setup.sh with independent components; copies of the deployed files in files/. ([source](https://github.com/kermes/surface-go-omarchy-setup))
- **Tested on:** Surface Go on Omarchy 4.x, Hyprland 0.56.2; ELAN touchscreen works on the stock kernel. ([source](https://github.com/kermes/surface-go-omarchy-setup))
- **With no keyboard at all:** Mostly. Two places still need a keyboard: the LUKS passphrase at boot (Plymouth has no touch input) and SDDM if autologin is off. Its fix is TPM2 unlock + autologin + lock at login, with a PCR caveat. ([source](https://github.com/kermes/surface-go-omarchy-setup/blob/main/07-gotchas.md))
- **Who made it:** kermes (github.com/kermes). ([source](https://github.com/kermes))
- **Licence:** No licence file reported by GitHub. _(unconfirmed)_
- **When:** Created 2026-09-21, last push 2026-10-07. ([source](https://github.com/kermes/surface-go-omarchy-setup))

### Surface Book 2 on Omarchy

- **What it gives a tablet:** Safe detaching with the discrete GPU in the base, touchscreen and pen through iptsd, tablet mode, on-screen keyboard, virtual trackpad, auto-rotation. ([source](https://github.com/Overpaul-Ops/surface-book-2-omarchy))
- **How it goes on:** install.sh in steps, with a reboot between; boot menu must be pointed at the linux-surface kernel or touch is lost. ([source](https://github.com/Overpaul-Ops/surface-book-2-omarchy))
- **Tested on:** Surface Book 2 on Omarchy 4.x. ([source](https://github.com/Overpaul-Ops/surface-book-2-omarchy))
- **With no keyboard at all:** Yes, detached clipboard with touch and pen. ([source](https://github.com/Overpaul-Ops/surface-book-2-omarchy))
- **Who made it:** Overpaul-Ops (github.com/Overpaul-Ops). ([source](https://github.com/Overpaul-Ops))
- **Licence:** MIT ([source](https://github.com/Overpaul-Ops/surface-book-2-omarchy))
- **When:** Created and last pushed 2026-09-29. ([source](https://github.com/Overpaul-Ops/surface-book-2-omarchy))

### Surface touch on the stock kernel

- **What it gives a tablet:** Surface Pro touchscreen and pen on Omarchy's stock kernel through an IPTS DKMS module plus iptsd, so linux-surface's whole kernel is not needed. ([source](https://github.com/ImNickRichard/surface-touch-omarchy))
- **How it goes on:** DKMS package from the repository. ([source](https://github.com/ImNickRichard/surface-touch-omarchy))
- **Tested on:** Surface Pro 7 only. ([source](https://x.com/NickElojire/status/2106600574787473521))
- **With no keyboard at all:** A driver, not a tablet layer: pair it with one of the tablet plugins. _(unconfirmed)_
- **Who made it:** Nick Richard (github.com/ImNickRichard, x.com/NickElojire), crediting the linux-surface developers for the driver work. ([source](https://x.com/NickElojire/status/2106600574787473521))
- **Licence:** GPL-2.0 ([source](https://github.com/ImNickRichard/surface-touch-omarchy))
- **When:** Created and last pushed 2026-10-04. ([source](https://github.com/ImNickRichard/surface-touch-omarchy))

### linux-surface

- **What it gives a tablet:** The kernel, firmware and iptsd touch daemon that make Surface touchscreens and pens work at all on IPTS models. ([source](https://github.com/linux-surface/linux-surface))
- **How it goes on:** Its own pacman repository; kermes adds it as a second Limine entry rather than the default. ([source](https://github.com/kermes/surface-go-omarchy-setup))
- **Tested on:** Its supported-devices wiki lists every Surface generation and what works on each. ([source](https://github.com/linux-surface/linux-surface/wiki/Supported-Devices-and-Features))
- **With no keyboard at all:** Hardware support only. _(unconfirmed)_
- **Who made it:** Maximilian Luz (qzed), Dorian Stoll (StollD, also iptsd's main author) and Jake Day (jakeday), the top contributors. ([source](https://github.com/linux-surface/linux-surface/graphs/contributors))
- **Licence:** GitHub reports no licence at the repository root; kernel patches carry the kernel's GPL-2.0. _(unconfirmed)_
- **When:** Since 2018; last push 2026-10-22; 7,837 stars. ([source](https://github.com/linux-surface/linux-surface))

## Native ARM tablets: Android or iOS replaced

### Amazon Fire Max 11

- **What it gives a tablet:** Native Omarchy on an Amazon Fire Max 11 on mainline Linux 7.2 with its own device tree: display, Panfrost GPU (Mali-G57), multi-touch and pen, Wi-Fi, speakers, battery, 15 W charging, auto-rotate, auto-brightness, an on-screen keyboard and tablet bar buttons. Claude Code runs on it. ([source](https://x.com/The_Mr_Wizard/status/2109547701490552990))
- **How it goes on:** No image or repository yet: 'a shareable flashable image' is the last item on its next-up list. _(unconfirmed)_
- **Tested on:** Fire Max 11 (2023, MT8188J). Missing: microphones, Bluetooth, suspend, USB-C host, GPU scaling, fuel gauge, RTC, video decode. ([source](https://x.com/The_Mr_Wizard/status/2109547701490552990))
- **With no keyboard at all:** Yes: it is a slate with no keyboard and runs Omarchy's lock screen and keyboard. ([source](https://x.com/The_Mr_Wizard/status/2109547701490552990))
- **Who made it:** The_Mr_Wizard (x.com/The_Mr_Wizard). ([source](https://x.com/The_Mr_Wizard/status/2109547701490552990))
- **Licence:** Unpublished. _(unconfirmed)_
- **When:** Posted 2026-10-12. ([source](https://omarchyarchive.com/posts/))

### iPad 6

- **What it gives a tablet:** Omarchy running natively on an iPad 6: touchscreen, Wi-Fi, hardware buttons, brightness, charging (capped at 425 mA) and battery work; Bluetooth and suspend in progress. ([source](https://x.com/henrikklee/status/2113026261417418965))
- **How it goes on:** 'Repo with install instructions, tooling, drivers and touch adapted shell (wip) coming soon.' _(unconfirmed)_
- **Tested on:** iPad 6 only. ([source](https://x.com/henrikklee/status/2113026261417418965))
- **With no keyboard at all:** Yes; its shell is being adapted for touch. ([source](https://x.com/henrikklee/status/2113026261417418965))
- **Who made it:** Henrik Klee (github.com/henrikklee). ([source](https://github.com/henrikklee))
- **Licence:** Unpublished. _(unconfirmed)_
- **When:** Posted 2026-10-21. ([source](https://x.com/henrikklee/status/2113026261417418965))

### $50 photo frame

- **What it gives a tablet:** Omarchy on a $50 Allwinner A33 photo frame from microSD: Hyprland, touch, on-screen keyboard, Wi-Fi, Bluetooth LE keyboard and mouse, with the original eMMC untouched. Proof that 1 GB of RAM and a Mali-400 can carry it. ([source](https://x.com/redp314/status/2109699021799293244))
- **How it goes on:** SD-card-only build for the PF1007L frame, reinstall steps in the repository. ([source](https://github.com/pangoleen/ormarch-picture-frame))
- **Tested on:** PF1007L / Allwinner A33 only. ([source](https://github.com/pangoleen/ormarch-picture-frame))
- **With no keyboard at all:** Yes, touch only. ([source](https://x.com/redp314/status/2109995906028179952))
- **Who made it:** redp314 (x.com/redp314), repository under github.com/pangoleen. ([source](https://github.com/pangoleen/ormarch-picture-frame))
- **Licence:** MIT ([source](https://github.com/pangoleen/ormarch-picture-frame))
- **When:** Published 2026-10-13. ([source](https://github.com/pangoleen/ormarch-picture-frame))

### Omarchy Mobile (Pixel phones)

- **What it gives a tablet:** Native Omarchy on a Pixel 7 Pro with a touch shell on top of unmodified Omarchy: phone home screen and launcher, on-screen keyboard, app switcher, PIN lock, quick settings. Mesa Panfrost gained the Mali-G710 for it. ([source](https://x.com/derekmross/status/2109678547010826592))
- **How it goes on:** tools/build-packages, build-rootfs, then install-phone, which writes only slot B's boot images and userdata; stock Android stays on slot A as the way back. Needs an unlocked Pixel 7 Pro and an Omarchy laptop with Docker. 'Expect rough edges.' ([source](https://github.com/derekross/omarchy-mobile))
- **Tested on:** Pixel 7 Pro; other phones are arriving as pull requests from other people. ([source](https://x.com/derekmross/status/2113338435108745600))
- **With no keyboard at all:** Yes, phone-sized. It is the touch shell a tablet port would borrow. ([source](https://github.com/derekross/omarchy-mobile))
- **Who made it:** Derek Ross (github.com/derekross, x.com/derekmross). ([source](https://github.com/derekross))
- **Licence:** MIT ([source](https://github.com/derekross/omarchy-mobile))
- **When:** Created 2026-10-12, last push 2026-10-23, 178 stars. ([source](https://github.com/derekross/omarchy-mobile))

## Android kept underneath

### omarchy-android

- **What it gives a tablet:** Rootless native ARM64 Omarchy and Hyprland inside Termux and Termux:X11, nothing flashed. ([source](https://github.com/BlackFireAlex/omarchy-android))
- **How it goes on:** One-command installer; prebuilt release around 1.18 GB. Requires Android 12 / API 31 unless overridden. ([source](https://github.com/BlackFireAlex/omarchy-android))
- **Tested on:** Adreno 840 (Turnip). A Galaxy Tab S9 Ultra report shows VirGL failing to start. ([source](https://github.com/BlackFireAlex/omarchy-android/issues/4))
- **With no keyboard at all:** Android's own touch and keyboard sit underneath; untested as a tablet desktop. _(unconfirmed)_
- **Who made it:** BlackFireAlex (github.com/BlackFireAlex). ([source](https://github.com/BlackFireAlex))
- **Licence:** MIT ([source](https://github.com/BlackFireAlex/omarchy-android))
- **When:** Created 2026-08-30, last push 2026-10-08. ([source](https://github.com/BlackFireAlex/omarchy-android))

### Omalauncher

- **What it gives a tablet:** An Android launcher in Omarchy's themes with optional Arch Linux ARM beside the phone apps, without root. ([source](https://github.com/SimonSchubert/omalauncher))
- **How it goes on:** Android app (GitHub and Google Play builds). ([source](https://github.com/SimonSchubert/omalauncher))
- **Tested on:** Phones; Android 10+ ARM64. ([source](https://github.com/SimonSchubert/omalauncher))
- **With no keyboard at all:** Keeps Android as the tablet; it is a look, not Omarchy's desktop. _(unconfirmed)_
- **Who made it:** Simon Schubert (github.com/SimonSchubert), also the author of moarchy for the PinePhone. ([source](https://github.com/SimonSchubert))
- **Licence:** GitHub reports NOASSERTION; bundled Termux binaries are GPL-2.0 / LGPL-3.0 / BSD-3-Clause per its README. _(unconfirmed)_
- **When:** Created 2026-10-02, last push 2026-10-22, 437 stars. ([source](https://github.com/SimonSchubert/omalauncher))

## Things noticed along the way

- Omarchy installs with full-disk encryption, and the passphrase prompt at boot has no touch input. On a slate with no keyboard this is the first wall, before any tablet plugin runs. ([source](https://github.com/kermes/surface-go-omarchy-setup/blob/main/07-gotchas.md))
- Upstream Omarchy's screensaver ignores touch: only a key press inside its terminal dismisses it. Open as omacom/omarchy issue 7762; a fix is proposed in PR 7528. ([source](https://github.com/omacom/omarchy/issues/7762))
- Omarchy's menu takes exclusive keyboard focus, so a tap on any on-screen keyboard closes it. Open upstream as issue 9029; Ragtop and Gimbal work around it by cloning the overlays. ([source](https://github.com/omacom/omarchy/issues/9029))
- The Omarchy Archive (omarchyarchive.com, github.com/jstamb/omarchy-archive) indexes 8,073 community records with device and form-factor fields; 92 of its posts concern tablets or touch. Its data is public JSON and is the fastest way to find who has already run Omarchy on a given model. ([source](https://github.com/jstamb/omarchy-archive))
- Eighteen tablet, keyboard, rotation and touch plugins are listed in Omarchy's marketplace as of 2026-10-23, added between 2026-08-29 and 2026-10-19. None is first-party: Omarchy itself ships no tablet mode. ([source](https://github.com/jstamb/omarchy-archive/blob/main/data/plugins.json))
