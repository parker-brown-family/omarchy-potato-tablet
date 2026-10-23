# Status: diagnostic kit

**Difficulty: 7/10** (scored at the start) — a loop through the physical world (a webcam on real glass), touch injected
on a device we only reach over a cable, and a verdict other people's agents will trust. The
expensive mistake is a "pass" that really meant "could not see", and it can stay hidden for a
long time. → **four gates**
**Turned out to be:** closer to 5 so far: Gates 1 and 2 came back approved with nothing changed, so Gates 3 and 4 were collapsed into one plan (`03-plan.md`).

- Gate 1 — Product: APPROVED 2026-10-23 (onn only for now; C920 fixed above the tablet; *type hello* is the tracer bullet; frames judged as recommended)
- Gate 2 — Architecture: APPROVED 2026-10-23 ("concur all")
- Gates 3 and 4 — Program design and slices: collapsed into `03-plan.md`, building

## Slices
- [x] Slice 1 — tracer bullet: type hello, replaying Parker's recorded touch points into a sink.
      Passed on the onn on 2026-10-23 with all four observers (`docs/examples/type-hello-first-pass/`).
      The first run left the webcam not observed at Jev 0.46; a 36-point sink font fixed it.
- [ ] Slice 2 — calibrate: four targets, fit the matrix from raw events
- [ ] Slice 3 — any word: key centres from wvkbd's layout tables
- [ ] Slice 4 — the rest of the task library
- [ ] Slice 5 — the skill, and a published example record

## Notes for a fresh session
- Parker's brief for this kit is his note on 2026-10-23, quoted in `01-product.md`. He concurred
  on Ragtop as the tablet layer and on an agent driving the tablet.
- The tablet is the onn 10.1" Tablet Pro, booting Omarchy natively. It has no Wi-Fi, so the only
  link is the USB serial gadget (`1d6b:0104`, serial `onn-native-0`), driven by `native-exec.py`
  in the private onn-omarchy workspace.
- The serial port is opened without sudo through a uaccess udev rule, `kit/udev/70-onn-tablet.rules`,
  installed 2026-10-23. The copy Parker pasted that day wrapped onto two lines and fails
  `udevadm verify`; its second line still tags the gadget, so access works until it is replaced.
- A Logitech C920 is attached to the laptop as `/dev/video2`, fixed above the tablet since
  2026-10-23. Its frames come out a quarter-turn from the tablet; `transpose=2` (90° counter-
  clockwise) makes them upright. Frames stay off the public repository.
- Gliff 0.4.0 is installed on the laptop. It forwards keyboard and pointer, not touch, and needs
  a network the tablet does not yet have.
