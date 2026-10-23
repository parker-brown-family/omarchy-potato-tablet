# Gate 2 — Architecture: the diagnostic kit

Gate 1 settled what the kit does. This page settles where each part lives, what talks to what,
and the one trap in the design that would otherwise ship green.

## The pieces, built up

**1 · The link, which exists today.** The laptop reaches the onn over a USB serial gadget. A
script goes over as base64, runs in the tablet's shell, and its output comes back between two
sentinel lines. `native-exec.py` in the private onn workspace does this now.

```
 LAPTOP                                   TABLET (onn, native Omarchy)
┌────────────────────┐   USB serial   ┌──────────────────────────────┐
│ potato             │◄──────────────►│ BusyBox shell                │
│  link: serial      │                │                              │
└────────────────────┘                └──────────────────────────────┘
```

**2 · Hands and the tablet's own eyes.** A small program on the tablet, `potato-agent`, writes
taps into the touchscreen's input device, records the raw events it sees, and dumps the
framebuffer. The laptop sends it commands over the same link.

```
┌────────────────────┐   USB serial   ┌──────────────────────────────┐
│ potato             │◄──────────────►│ potato-agent                 │
│  link: serial      │  tap, record,  │  tap ──► /dev/input (touch)  │
│                    │  framebuffer   │  record ◄── evdev events     │
│                    │                │  dump ◄── /dev/fb0           │
└────────────────────┘                └──────────────────────────────┘
```

**3 · Two more observers.** The webcam on the laptop watches the glass. On the tablet, a *sink*
is a terminal the kit opens itself, which writes every character it receives to a file. The sink
is new since Gate 1: it is the one observer that says exactly what an application received.

```
┌────────────────────┐   USB serial   ┌──────────────────────────────┐
│ potato             │◄──────────────►│ potato-agent                 │
│  link: serial      │                │  tap · record · dump         │
│  webcam ◄─ C920 ───┼── looks at ───►│  sink: foot ──► typed.txt    │
└────────────────────┘    the glass   └──────────────────────────────┘
```

**4 · Judging, the record and the page.** Capture writes evidence and stops. A separate judge
stage reads the evidence and answers per observer: exact checks where the expectation is exact,
and Jev, through `JevClient`, on webcam frames. The record is the contract between the stages,
and the page is drawn from the record, never written by hand. When a task needs hands, `potato`
asks the person on the laptop and waits.

```
┌────────────────────┐   USB serial   ┌──────────────────────────────┐
│ potato             │◄──────────────►│ potato-agent                 │
│  link · webcam     │                │  tap · record · dump · sink  │
│  ask the person    │                └──────────────────────────────┘
└─────────┬──────────┘
          │ evidence: frames, events, what was typed
          ▼
┌────────────────────┐    ┌────────────────────┐    ┌──────────────────┐
│ record.json        │───►│ judge (JevClient)  │───►│ run page (HTML)  │
│  the contract      │    │  answers per       │    │  drawn from the  │
│                    │    │  observer          │    │  record          │
└────────────────────┘    └────────────────────┘    └──────────────────┘
```

## The four observers

| Observer | Lives | Sees | Judged by |
|---|---|---|---|
| input events | tablet, evdev | what the kernel delivered | exact: downs and ups, coordinates inside the target |
| sink | tablet, a kit-owned terminal | what the application received | exact: the typed string |
| framebuffer | tablet, `/dev/fb0` | what software drew | exact region change; Jev where meaning is needed |
| webcam | laptop, the C920 | what the glass shows | Jev on the rectified frame, with the colour canary |

Each answers pass, fail, not observed or not attempted on its own. Nothing sums them.

## The trap: aiming with the thing under test

A synthetic tap is written in the digitizer's coordinates. To hit a key, the kit has to turn a
screen position into a digitizer position, which needs the calibration. **If the kit aimed
with the compositor's own transform, a wrong transform would aim wrong in exactly the way that
cancels out, and every tap would still land on its key.** Synthetic taps cannot test the
calibration. Only a finger on the glass can.

So there are two kinds of task:

| Kind | Who acts | What it proves | Example |
|---|---|---|---|
| **calibration** | the person | the mapping from glass to digitizer, fitted from where they tapped and what the digitizer reported, with the webcam confirming the finger was on the target | tap the four dots |
| **regression** | the agent | the software path, given a calibration | type hello |

Every regression result names the calibration it aimed with and where that calibration came
from. Today that is transform 3, from Parker's physical `hello` on 2026-10-22.

## Geometry the kit needs

| What | Source | Checked against |
|---|---|---|
| key centres | wvkbd's own layout tables, at the panel's size | one framebuffer capture: the key drawn at each centre |
| webcam to framebuffer | the screen's four corners found in a webcam frame, solved once per mounting | every run: the rectified frame must match the framebuffer, or the webcam answers *not observed* |
| webcam orientation | measured 2026-10-23: frames need a 90° counter-clockwise turn | the same check |

## Links

| Link | For | State |
|---|---|---|
| serial | the onn: base64 transfer and sentinel lines, rewritten in Rust from `native-exec.py` | the port needs the uaccess udev rule first |
| SSH | tablets with a network | later |
| Gliff | a person watching a networked Hyprland tablet | not part of the kit's data path |

A **tablet profile** holds everything device-specific: how to find the link, the touchscreen's
node and axis ranges, the framebuffer's size, row stride and pixel format, the keyboard and its
layout, the current calibration with its provenance, and the webcam's device, rotation and
geometry. Adding a tablet means adding a profile, not changing code.

## Where it lives

```
omarchy-potato-tablet/
  kit/                     Rust workspace
    potato/                laptop: runs tasks, captures the webcam, writes the record and page
    potato-agent/          tablet: tap, record, dump, sink (static ARM64)
    potato-record/         the record format, shared by both
  judge/                   the judge stage, on JevClient
  tasks/                   the task library, as data
  profiles/onn-10-pro.toml the onn's profile
  skills/potato-diagnostics/SKILL.md   the procedure an agent follows
```

## Lines this design holds

- **Webcam frames stay on the laptop.** They can show a room or a face. The public repository
  carries code, tasks, profiles and example records with framebuffer evidence only.
- **The Jev key stays on the laptop**, read from jev-integrations' `.env` as everything else does.
- **No flashing.** The kit never writes a boot partition.
- **An answer has no default.** A record field that was never measured reads as missing, not as
  pass and not as zero.

## Left for Gate 3

How targets are drawn for calibration tasks, the record's exact fields, the Jev questions
(worded with `ask-jev`), how `potato-agent` is built (a static musl binary that runs outside the
Arch root, or a glibc one inside it), and the task file format.

## Questions for Parker

1. **Physical calibration, synthetic regression.** A person's taps fit the calibration, the agent's
   taps test everything else against it. Right split?
2. **The sink.** A fourth observer, a terminal the kit opens that records what it was sent. In?
3. **Capture in Rust, judging in a separate stage on JevClient.** It means old runs can be judged
   again, and the record format is the only contract. JevClient is Python, so the judge is too.
4. **Webcam frames never go public.** Agreed?
