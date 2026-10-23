# omarchy-potato-tablet

Turning an old tablet into a working [Omarchy](https://omarchy.org) machine: what we did on ours,
what other people have done on theirs, and the tools both used.

## Our tablet

An onn 10.1" Tablet Pro from about 2020, MediaTek MT8768, that shipped with Android 11. It now
boots Omarchy 4.0.6 natively, with nothing underneath.

| | State on 2026-10-22 |
|---|---|
| Desktop: Hyprland, the Omarchy shell, Foot | boots straight to it, and checks itself |
| On-screen keyboard and touch typing | works, calibrated |
| Graphics | software only, capped at 10 Hz: the PowerVR GPU has no open driver |
| Wi-Fi, audio, battery level | not yet |
| Passwords between power-on and the desktop | none |

The details are in [our tablet](docs/our-tablet.md) and the order we did it in is in
[how we did it](docs/workflow.md).

## Start here

| Page | What it has |
|---|---|
| [Our tablet](docs/our-tablet.md) | what runs, what works, what doesn't, how it boots |
| [How we did it](docs/workflow.md) | the steps in order, and the tools |
| [Gotchas](docs/gotchas.md) | what cost time, for Android and x86 tablets, with the commands |
| [Other people's work](docs/ecosystem.md) | 19 tablet projects for Omarchy, each read at its source |
| [Who made this possible](docs/people.md) | the people behind every piece |

## Which route fits your tablet

| Your tablet ran | Route | Who has done it |
|---|---|---|
| Android, and you will replace it | a native port: your tablet's own kernel and device tree, or mainline | onn (us), Amazon Fire Max 11, Pixel phones |
| Android, and you will keep it | Omarchy inside Android, no root | omarchy-android, Omalauncher |
| Windows, on Intel or AMD | install the Omarchy ISO, then add a tablet layer such as Ragtop | Surface Go, Pro and Book, Lenovo Yoga, Framework 12 |
| iOS | a native port, one model at a time | iPad 6 |

## Coming

- A diagnostic kit that tests a tablet by itself: it taps the screen through a virtual touch
  device, then checks what changed in the tablet's own screenshot and through a webcam
  ([plan](docs/plans/diagnostic-kit/00-status.md)).
- The onn port's code, once it has had a publishing pass.

## Licence

MIT, for everything in this repository. Linked projects carry their own licences.
