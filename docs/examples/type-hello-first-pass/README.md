# Type hello, the first pass on the onn

The kit's first run on real hardware where all four observers passed, on 2026-10-23. Nobody touched
the tablet: the five taps replayed Parker's recorded touch-downs from the night before, into a
terminal the kit opened itself.

| Observer | Answer | What it saw |
|---|---|---|
| input events | pass | five touch-downs, each exactly where it was aimed |
| sink | pass | the terminal received exactly `hello` |
| framebuffer | pass | Jev read the changed region as showing the word at 0.98, and the same region before at 0.02 |
| webcam | pass | the same spot of glass, through a C920 above the tablet: 0.98 after, 0.02 before |

`record.json` is the run record as the judge left it. The two framebuffer crops are here; the
webcam frames are not, because webcam frames stay on the laptop.

The run before this one passed three observers and left the webcam *not observed*: at the desktop's
12-point font the word came through the camera at 0.46, between the thresholds, and the judge
handed it to a person rather than guess. The sink now types at 36 points.
