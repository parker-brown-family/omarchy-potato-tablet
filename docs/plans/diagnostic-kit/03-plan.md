# Gates 3 and 4 — Program design and slices, as one plan

Gates 1 and 2 both came back approved with nothing changed, which says the score of 7 was too
high. The rest is collapsed into this page, and building starts from it.

## The aim, settled by the record

The tracer bullet does not compute where h, e, l, l and o are. On 2026-10-22, after the final
reboot, Parker typed `hello` with his finger and the tablet recorded every raw event. Decoded,
his five touch-downs were, in the digitizer's own coordinates:

| Key | x | y |
|---|---|---|
| h | 104 | 998 |
| e | 144 | 563 |
| l | 97 | 1417 |
| l | 102 | 1401 |
| o | 137 | 1363 |

All five agree with the saved quarter-turn (screen x = y / 1919 × 1200, screen y =
(1 − x / 1199) × 1920). The task replays these points. Its aim comes from a physical calibration,
as Gate 2 requires, and the record says so.

## Program design

**`kit/potato-agent`** — Rust, static ARM64 (`aarch64-unknown-linux-musl`, linked with rust-lld,
checked under qemu). Two commands:

```
potato-agent tap --dev /dev/input/event3 --at 104,998 --at 144,563 … [--hold-ms 60] [--gap-ms 300]
potato-agent record --dev /dev/input/event3 --seconds 8 --out /tmp/potato/events.bin
```

`tap` writes the same multitouch sequence the panel produces: tracking id, position, touch size,
pressure, BTN_TOUCH, sync, then release. `record` copies raw events for a bounded time.

**`kit/potato`** — Rust, on the laptop:

```
potato run tasks/type-hello.toml --profile profiles/onn-10-pro.toml --out runs/<id>
```

It finds the tablet's serial gadget by vendor, product and serial in sysfs, sends files and
scripts with the same base64 and sentinel protocol as `native-exec.py`, and for each task:
captures the framebuffer and a webcam frame, opens the sink, starts recording, taps, then captures
again, reads back what the sink received, and writes `record.json` with every answer still empty.

**`judge/judge.py`** — Python, standard library plus `JevClient`, with ffmpeg for frames:

```
python3 judge/judge.py runs/<id>
```

It fills in each observer's answer and writes `runs/<id>/index.html`. Exact checks: the sink's
text, the number of taps and where they landed. Jev checks, worded with `ask-jev`: the framebuffer
and the upright webcam frame. A webcam frame that does not match the framebuffer's layout answers
*not observed*.

**The record** — `potato-record`, shared types serialised as JSON. An answer is an enum with no
default (`pass`, `fail`, `not_observed`, `not_attempted`), wrapped in an option, so a field nobody
judged is missing and never reads as pass.

**Task and profile files** are TOML. A task names its kind (calibration or regression), its act
(taps with their source), and what each observer expects. The profile holds the link, the
touchscreen node and ranges, the framebuffer geometry, the calibration with its provenance, and
the webcam's device and rotation.

## Slices

1. **Type hello, end to end.** The tracer bullet: replay Parker's recorded points into a sink,
   four observers, a judged record and its page.
2. **Calibrate.** Draw four targets, ask the person to tap them, fit the matrix from the raw
   events, and compare it with the profile's.
3. **Any word.** Key centres from wvkbd's layout tables, checked against a framebuffer capture,
   so a task can type something other than what Parker typed.
4. **The rest of the library.** Keyboard comes up, open an app, boot to desktop, and the
   *not attempted* rows for rotate, wake and unlock.
5. **The skill.** `skills/potato-diagnostics/SKILL.md`, and a published example record with
   framebuffer evidence only.
