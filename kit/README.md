# The diagnostic kit

Tests a tablet by itself: taps the screen through the touchscreen's own input device, then checks
what changed through four observers. The plan is in `docs/plans/diagnostic-kit/`.

| Observer | Sees | Judged by |
|---|---|---|
| input events | what the kernel delivered | exact |
| sink | what a terminal the kit opened received | exact |
| framebuffer | what software drew | a diff inside the sink window, then Jev on the crop, with the "before" crop as a control |
| webcam | what the glass shows | Jev on the same spot of the glass, with the same control |

Each answers `pass`, `fail`, `not_observed` or `not_attempted`, and nothing adds them together.
A synthetic tap proves everything above the digitizer and nothing about the glass.

## Build

```sh
rustup target add aarch64-unknown-linux-musl
cargo build --release -p potato-agent --target aarch64-unknown-linux-musl   # the tablet side, static
cargo build --release -p potato                                             # the laptop side
cargo test
```

## Check the link without a tablet

The serial protocol runs against any shell on a pseudo-terminal, including a tablet's own BusyBox
under qemu:

```sh
socat PTY,link=/tmp/potato-tty,raw,echo=0 EXEC:"qemu-aarch64-static busybox.static sh -i",pty,setsid,ctty,stderr &
target/release/potato selftest --port /tmp/potato-tty --out /tmp/potato-selftest
```

## Run a task, then judge it

```sh
target/release/potato run ../tasks/type-hello.toml --profile ../profiles/onn-10-pro.toml \
    --agent target/aarch64-unknown-linux-musl/release/potato-agent --out ../runs/<id>
python3 ../judge/judge.py ../runs/<id> --profile ../profiles/onn-10-pro.toml
```

The judge needs `jev-integrations` beside this repository (or `JEV_INTEGRATIONS` set) for
`JevClient` and its key. Without a key, `--no-jev` runs the exact checks and answers the picture
observers `not_observed`. Runs stay out of git: webcam frames can show a room.

## Walk through an app

A journey is several steps inside one app, each a synthetic touch and the outcome it should cause.
`journeys/lampada-walk.toml` is the worked example: nine steps through Orthodox Lampada, a children's
web app of ours that runs on the tablet. It opens the calendar and a day, goes back twice, opens the
saints, types `lion` on the on-screen keyboard, scrolls the results, then opens a saint and closes the
panel.

```sh
target/release/potato journey ../journeys/lampada-walk.toml --profile ../profiles/onn-10-pro.toml \
    --agent target/aarch64-unknown-linux-musl/release/potato-agent --out ../runs/<id> --film
python3 ../judge/judge.py ../runs/<id> --profile ../profiles/onn-10-pro.toml
python3 ../scripts/film-walk.py ../runs/<id> --profile ../profiles/onn-10-pro.toml
```

Nothing is aimed from recorded points. In the app, the page says where the element is, through
Chromium's DevTools port on the tablet's loopback (`potato-agent cdp`), and whether a tap at its centre
would reach it. If something covers it or it is below the fold, the step is not run. On the on-screen
keyboard, `potato-agent keys` finds the keys in the framebuffer as they are drawn now, and the profile's
layout labels them by order. The calibration matrix then carries each screen point into the
touchscreen's coordinates. A step can `swipe` instead of tapping, to scroll.

| Observer | Sees | Judged by |
|---|---|---|
| input events | what the kernel delivered | exact, starting from where the touch slot already stood |
| app state | what the app says happened, a JavaScript check asked after the act | exact: only `true` passes |
| framebuffer | what software drew, inside the app's window | Jev, asked whether the step's description fits, with "before" as a control |
| webcam | what the glass shows | the same question, on the same spot of the glass |

`--film` records the webcam for the whole walk and cuts each step's stills from the film at the
moment they were taken. It also writes `film.json`, the time of every tap in the film.
`scripts/film-walk.py` turns a filmed, judged run into footage: the camera film upright, and a
split screen. The split has the framebuffer on the left with a ring where each synthetic finger landed,
the camera on the right, and each step's answers as captions.

On the onn, the app is served on the tablet's own loopback by a boot hook, and opened by [`onn-webapp`](../docs/onn/bin/onn-webapp), whose
[Chromium flags](../docs/onn/bin/chromium-flags.conf) turn on the loopback DevTools port.

The serial port needs read and write access. On Linux, tag the tablet's gadget for the logged-in
user. The onn's rule is a file, so it cannot wrap when pasted:

```sh
sudo cp udev/70-onn-tablet.rules /etc/udev/rules.d/ && sudo udevadm control --reload && sudo udevadm trigger --subsystem-match=tty
```
