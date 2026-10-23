# Gate 1 — Product: the diagnostic kit

## The problem

Bringing up a tablet is a loop: change something, reboot, have a person tap the screen and say
what happened. The person is the slow part. On 2026-10-22 the onn's touch calibration waited on
Parker coming back from dinner to type `hello`, and that one word was the only test that could
tell a flipped screen from a rotated one.

Parker, 2026-10-23:

> FULLY automating this process as much as possible by SIMULATING AND SCREEN SHOTTING / recording
> / webcam to screenshot the tablet while SIMULATING touches to determine what happens would be
> the SICKEST tool for an agent to use … press q, and type hello on the keyboard to calibrate the
> touchscreen would be done by the agent on the mainbox. It would do this by grabbing the webcam
> and taking SCREENSHOTS at appropriate times to get before and after images of the CALIBRATING
> action … THEN when the person uses it - the simulated tests are done AND THE DATA IS COLLECTED
> meaning that the loop will be much tighter for the user offering feedback.

## Who uses it

1. **An agent bringing up a tablet**, working alone for long stretches.
2. **The person holding the tablet**, who is asked only for what needs hands: power, turning it,
   a first boot, a real finger on the glass.
3. **Someone else's agent** on someone else's potato tablet, following this repository.

## What one task run does

Take *type hello*:

1. **Before.** Capture the tablet's own framebuffer (what software drew) and a webcam frame (what
   the glass shows).
2. **Act.** Inject taps at the centres of h, e, l, l, o through the touchscreen's input device.
   For an act software cannot do, such as turning the tablet over, ask the person and wait.
3. **After.** Capture both again, plus the raw input events the tablet saw.
4. **Judge.** Each observer answers on its own: did the expected change happen?
5. **Record.** One JSON record, and one page with the before and after frames side by side.

## Four answers, not two

| Answer | Means |
|---|---|
| pass | the observer saw the expected change |
| fail | the observer saw something else |
| not observed | the observer could not look: no webcam, a dark frame, a dropped link |
| not attempted | the act never ran |

**A synthetic pass is not a physical pass.** Synthetic taps enter at the kernel's input device, so
they prove everything above the digitizer and nothing about the glass. Every record says which
input was used, and a summary never adds the two together.

## The first task library

| Task | Act | By | Expected |
|---|---|---|---|
| Boot to desktop | reboot from the serial shell, or power on | agent, or person | the boot check passes; the webcam sees the desktop |
| Type hello | taps at the key centres | agent | Foot shows `hello` |
| Calibration sweep | taps at a grid of known points | agent | each tap lands in the region drawn there |
| Keyboard comes up | tap a text field | agent | the keyboard is drawn |
| Open an app | tap a launcher entry | agent | a new window maps |
| Rotate | turn the tablet, or set a transform | person, or agent | screen and touch turn together; a tap still lands |
| Wake | press power, or jiggle | person | the screen comes back |
| Unlock | type a PIN or password | agent | the lock screen is gone |

On the onn today, rotation, wake and unlock cannot pass, because rotation and suspend are untested
and there is no lock screen. Those rows record `not attempted` there rather than disappearing.

## What already exists

The onn bring-up built most of the pieces by hand. The kit makes them a loop.

| Piece | Today |
|---|---|
| run a command on the tablet, keep the receipt | `native-exec.py`, over USB serial |
| the tablet's own screenshot | `capture-framebuffer.sh` and `decode-framebuffer.py` |
| a synthetic tap | `touch-info --synthetic-q`, which writes into the touchscreen's input device |
| raw input events | `record-touch.sh` and `read-touch-recording.sh` |
| boot health | `verify-boot.sh` and the 60-second desktop check |
| a webcam | a Logitech C920 on the laptop, not yet pointed at the tablet |

## Ways in

| Link | When |
|---|---|
| USB serial | the onn today: it has no network |
| SSH | x86 tablets, or the onn once it has Wi-Fi |
| Gliff | watching and typing on a Hyprland tablet with a network. Gliff forwards keyboard and pointer, not touch, so touch stays the kit's job. Touch forwarding in Gliff itself is a possible upstream contribution, subject to its contributing rules |

## Form

A **skill first**: the procedure an agent follows, the task library, how to judge a frame. Then a
**small program** for the parts that must be exact: injecting taps, capturing frames, writing the
record. Parker asked for Rust or a C dialect. The injector has to run on the tablet as a static
ARM64 binary.

## Out of scope

- A robot finger. Physical acts stay with the person.
- Testing applications, or judging how the desktop looks.
- Writing the boot partition. Flashing stays a step a person approves.

## Done when

- *Type hello* runs with nobody at the tablet, and its record holds before and after frames from
  the framebuffer and the webcam.
- When a person next tests the tablet by hand, the simulated results for the same tasks are
  already waiting beside their report.

## Questions for Parker

1. **Is the onn the only tablet for now?** Recommendation: yes. It is plugged in, and x86 tablets
   arrive once a second device is in hand.
2. **Can the C920 be fixed above the tablet?** The webcam observer is only as good as its angle,
   and it has to stay put between runs.
3. **Is *type hello* the tracer bullet?** Recommendation: yes. It is the test that needed a person
   yesterday, and it touches every part: link, injection, both captures, judging, the record.
4. **How should webcam frames be judged?** A pixel diff breaks on glare and angle. Recommendation,
   for Gate 2 to settle: exact checks on the framebuffer, and Jev on the webcam frame through
   `JevClient`'s image route, asking questions such as *"does the screen show a keyboard along its
   bottom edge?"*

[Mockup of a run's page](mockups/run-report.html)
