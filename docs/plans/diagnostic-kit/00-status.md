# Status: diagnostic kit

**Difficulty: 7/10** — a loop through the physical world (a webcam on real glass), touch injected
on a device we only reach over a cable, and a verdict other people's agents will trust. The
expensive mistake is a "pass" that really meant "could not see", and it can stay hidden for a
long time. → **four gates**
**Turned out to be:** _filled in at the end_

- Gate 1 — Product: APPROVED 2026-10-23 (onn only for now; C920 fixed above the tablet; *type hello* is the tracer bullet; frames judged as recommended)
- Gate 2 — Architecture: in progress
- Gate 3 — Program Design: pending
- Gate 4 — Slice plan: pending

## Slices
- [ ] Slice 1 — tracer bullet: "type hello" run end to end by an agent, with before and after
      frames from the tablet and the webcam

## Notes for a fresh session
- Parker's brief for this kit is his note on 2026-10-23, quoted in `01-product.md`. He concurred
  on Ragtop as the tablet layer and on an agent driving the tablet.
- The tablet is the onn 10.1" Tablet Pro, booting Omarchy natively. It has no Wi-Fi, so the only
  link is the USB serial gadget (`1d6b:0104`, serial `onn-native-0`), driven by `native-exec.py`
  in the private onn-omarchy workspace.
- An agent cannot open that serial port without sudo until Parker installs a udev rule tagging
  the gadget `uaccess`. Until then, nothing in this plan has run on the tablet.
- A Logitech C920 is attached to the laptop as `/dev/video2`, fixed above the tablet since
  2026-10-23. Its frames come out a quarter-turn from the tablet; `transpose=2` (90° counter-
  clockwise) makes them upright. Frames stay off the public repository.
- Gliff 0.4.0 is installed on the laptop. It forwards keyboard and pointer, not touch, and needs
  a network the tablet does not yet have.
