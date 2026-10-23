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

The serial port needs read and write access. On Linux, tag the tablet's gadget for the logged-in
user. The onn's rule is a file, so it cannot wrap when pasted:

```sh
sudo cp udev/70-onn-tablet.rules /etc/udev/rules.d/ && sudo udevadm control --reload && sudo udevadm trigger --subsystem-match=tty
```
