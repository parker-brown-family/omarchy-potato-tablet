# omarchy-potato-tablet

Turning an old tablet into a working [Omarchy](https://omarchy.org) machine: what we did on ours,
what other people have done on theirs, and the tools both used.

## Our tablet

An onn 10.1" Tablet Pro from about 2020, MediaTek MT8768, that shipped with Android 11. It now
boots Omarchy 4.0.6 natively, with nothing underneath.

| | State on 2026-10-23 |
|---|---|
| Desktop: Hyprland, the Omarchy shell, Foot | boots straight to it in about 30 seconds (filmed), and checks itself |
| Apps, by touch | a children's web app walked through in nine touches, typing and a scroll included, each checked by the app, the screen and a webcam |
| Wi-Fi and SSH | up about 20 seconds after power-on, every boot |
| Tablet layer | Ragtop, in tablet mode, set up by the kit's own synthetic taps |
| Battery level | real, from the vendor fuel-gauge daemon |
| Packages | `pacman` over Wi-Fi |
| Graphics | software only, capped at 10 Hz: the PowerVR GPU has no open driver |
| Speakers, rotation, suspend | not yet |
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

## The diagnostic kit

[The kit](kit/README.md) tests a tablet by itself. It touches the screen through the touchscreen's
own input device, aimed at where things are drawn at that moment, then checks the result four ways:
the kernel's events, the app's own state, the tablet's screenshot and a webcam above the glass.
Its plan and progress are in [the status page](docs/plans/diagnostic-kit/00-status.md).

## Coming

- The onn port's code, once it has had a publishing pass.

## Licence

MIT, for everything in this repository. Linked projects carry their own licences.
