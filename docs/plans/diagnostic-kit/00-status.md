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
- [ ] Slice 2 — calibrate: four targets, fit the matrix from raw events (waits for Parker's finger)
- First regression caught, 2026-10-23: after Ragtop was installed, type-hello failed with the sink
  receiving `nd..l`. Every tap landed one row low, because Ragtop's keyboard handle reserves a strip at
  the bottom and wvkbd moved up 36 px (row 1512 to 1476). The replay's premise, keys where Parker typed
  them, no longer holds; slice 3 (aim from the live layout) is the fix, or retiring wvkbd for Ragtop's
  keyboard. Run: `runs/type-hello-20261023-114724` (local).
- [x] Slice 3 — aim from the live layout. Done 2026-10-23, not as planned: key centres are found in
      the framebuffer each time (`potato-agent keys`, rows of 14, 14, 14, 14, 13 and 10 keys on the onn)
      rather than read from wvkbd's tables, and a web app's elements are placed by the page itself
      through Chromium's DevTools port (`potato-agent cdp`). Parker asked for the tests to run *inside* a
      program built for a tablet, so the slice became journeys (`potato journey`): Orthodox Lampada
      walked through in nine steps, with a swipe, typing and an app-state observer. Take 4
      (`runs/lampada-walk-20261023-130929`, local) answered pass on all 36 observers.
      Found on the way: the kernel drops a touch coordinate equal to the slot's last one, so the
      recorder now notes where the slot stood (two false fails in take 1). "A list of saints … with no
      panel open over it" read 0.79 and 0.72, under the 0.8 bar both times; describing what comes back
      instead read 0.95.
      Still open: `type-hello` replays recorded points and still fails since Ragtop moved the keyboard;
      it should aim with `potato-agent keys` too.
- [ ] Slice 4 — the rest of the task library
- [ ] Slice 5 — the skill, and a published example record

## Notes for a fresh session
- Parker's brief for this kit is his note on 2026-10-23, quoted in `01-product.md`. He concurred
  on Ragtop as the tablet layer and on an agent driving the tablet.
- The tablet is the onn 10.1" Tablet Pro, booting Omarchy natively. The kit's link is the USB serial
  gadget (`1d6b:0104`, serial `onn-native-0`). Since 2026-10-23 it also has Wi-Fi and SSH at every
  boot (`ssh onn`), which the kit does not need.
- The serial port is opened without sudo through a uaccess udev rule, `kit/udev/70-onn-tablet.rules`,
  installed by an agent with sudo on 2026-10-23 and passing `udevadm verify`. (Agents run sudo
  themselves on this box; nothing in the kit's setup is handed to a person to paste.)
- A Logitech C920 is attached to the laptop as `/dev/video2`, fixed above the tablet since
  2026-10-23. Its frames come out a quarter-turn from the tablet; `transpose=2` (90° counter-
  clockwise) makes them upright. Frames stay off the public repository.
- Gliff 0.4.0 is installed on the laptop. It forwards keyboard and pointer, not touch, and needs
  a network the tablet does not yet have.
