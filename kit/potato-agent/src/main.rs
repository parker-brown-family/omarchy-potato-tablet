//! potato-agent: the kit's hands on the tablet.
//!
//! `tap` writes synthetic touches into the touchscreen's own input device, in the same
//! multitouch sequence a finger produces on the onn's Himax panel (recorded on 2026-10-22):
//! touch size, pressure, position, tracking id, BTN_TOUCH, sync, then the release. Writing into
//! the device node means everything above the digitizer sees the tap, and nothing about the
//! glass is tested.
//!
//! `record` copies raw input events from a device to a file for a bounded time and size.
//!
//! Standard library only, so it builds as a static aarch64-unknown-linux-musl binary.

use std::fs::{File, OpenOptions};
use std::io::{ErrorKind, Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::process::ExitCode;
use std::thread::sleep;
use std::time::{Duration, Instant};

const EV_SYN: u16 = 0;
const EV_KEY: u16 = 1;
const EV_ABS: u16 = 3;
const SYN_REPORT: u16 = 0;
const BTN_TOUCH: u16 = 330;
const ABS_MT_SLOT: u16 = 47;
const ABS_MT_TOUCH_MAJOR: u16 = 48;
const ABS_MT_WIDTH_MAJOR: u16 = 50;
const ABS_MT_POSITION_X: u16 = 53;
const ABS_MT_POSITION_Y: u16 = 54;
const ABS_MT_TRACKING_ID: u16 = 57;
const ABS_MT_PRESSURE: u16 = 58;
const O_NONBLOCK: i32 = 0o4000;
const EVENT_SIZE: usize = 24; // struct input_event on 64-bit Linux
const RECORD_CAP: usize = 4 * 1024 * 1024;

fn event(kind: u16, code: u16, value: i32) -> [u8; EVENT_SIZE] {
    // The kernel stamps injected events itself, so the timeval stays zero.
    let mut e = [0u8; EVENT_SIZE];
    e[16..18].copy_from_slice(&kind.to_le_bytes());
    e[18..20].copy_from_slice(&code.to_le_bytes());
    e[20..24].copy_from_slice(&value.to_le_bytes());
    e
}

fn frame(dev: &mut File, events: &[(u16, u16, i32)]) -> std::io::Result<()> {
    let mut buf = Vec::with_capacity((events.len() + 1) * EVENT_SIZE);
    for &(k, c, v) in events {
        buf.extend_from_slice(&event(k, c, v));
    }
    buf.extend_from_slice(&event(EV_SYN, SYN_REPORT, 0));
    dev.write_all(&buf)
}

fn tap(dev_path: &str, points: &[(i32, i32)], hold: Duration, gap: Duration) -> Result<(), String> {
    let mut dev = OpenOptions::new().write(true).open(dev_path).map_err(|e| format!("open {dev_path}: {e}"))?;
    for (i, &(x, y)) in points.iter().enumerate() {
        let id = 1000 + i as i32;
        frame(&mut dev, &[
            (EV_ABS, ABS_MT_SLOT, 0),
            (EV_ABS, ABS_MT_TOUCH_MAJOR, 30),
            (EV_ABS, ABS_MT_WIDTH_MAJOR, 30),
            (EV_ABS, ABS_MT_PRESSURE, 30),
            (EV_ABS, ABS_MT_POSITION_X, x),
            (EV_ABS, ABS_MT_POSITION_Y, y),
            (EV_ABS, ABS_MT_TRACKING_ID, id),
            (EV_KEY, BTN_TOUCH, 1),
        ]).map_err(|e| format!("tap {i} down: {e}"))?;
        sleep(hold);
        frame(&mut dev, &[(EV_ABS, ABS_MT_SLOT, 0), (EV_ABS, ABS_MT_TRACKING_ID, -1), (EV_KEY, BTN_TOUCH, 0)])
            .map_err(|e| format!("tap {i} up: {e}"))?;
        println!("POTATO_TAP {i} {x} {y}");
        if i + 1 < points.len() {
            sleep(gap);
        }
    }
    Ok(())
}

fn record(dev_path: &str, seconds: u64, out_path: &str) -> Result<(), String> {
    let mut dev = OpenOptions::new().read(true).custom_flags(O_NONBLOCK).open(dev_path)
        .map_err(|e| format!("open {dev_path}: {e}"))?;
    let mut out = File::create(out_path).map_err(|e| format!("create {out_path}: {e}"))?;
    let deadline = Instant::now() + Duration::from_secs(seconds);
    let mut buf = [0u8; EVENT_SIZE * 64];
    let mut total = 0usize;
    while Instant::now() < deadline && total < RECORD_CAP {
        match dev.read(&mut buf) {
            Ok(0) => sleep(Duration::from_millis(5)),
            Ok(n) => {
                let n = n.min(RECORD_CAP - total);
                out.write_all(&buf[..n]).map_err(|e| format!("write: {e}"))?;
                total += n;
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock => sleep(Duration::from_millis(5)),
            Err(e) => return Err(format!("read {dev_path}: {e}")),
        }
    }
    println!("POTATO_RECORDED {} bytes {} events", total, total / EVENT_SIZE);
    Ok(())
}

fn parse_point(s: &str) -> Result<(i32, i32), String> {
    let (x, y) = s.split_once(',').ok_or_else(|| format!("expected X,Y, got {s}"))?;
    Ok((x.trim().parse().map_err(|_| format!("bad x in {s}"))?, y.trim().parse().map_err(|_| format!("bad y in {s}"))?))
}

fn usage() -> ExitCode {
    eprintln!("usage:\n  potato-agent tap --dev DEV --at X,Y [--at X,Y ...] [--hold-ms N] [--gap-ms N]\n  potato-agent record --dev DEV --seconds N --out FILE");
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(cmd) = args.first().cloned() else { return usage() };
    let mut dev = None;
    let mut points = Vec::new();
    let mut hold_ms = 80u64;
    let mut gap_ms = 300u64;
    let mut seconds = None;
    let mut out = None;
    let mut it = args[1..].iter();
    while let Some(flag) = it.next() {
        let Some(val) = it.next() else { return usage() };
        let parsed = match flag.as_str() {
            "--dev" => { dev = Some(val.clone()); Ok(()) }
            "--at" => parse_point(val).map(|p| points.push(p)),
            "--hold-ms" => val.parse().map(|v| hold_ms = v).map_err(|_| "bad --hold-ms".into()),
            "--gap-ms" => val.parse().map(|v| gap_ms = v).map_err(|_| "bad --gap-ms".into()),
            "--seconds" => val.parse().map(|v: u64| seconds = Some(v.min(600))).map_err(|_| "bad --seconds".into()),
            "--out" => { out = Some(val.clone()); Ok(()) }
            _ => return usage(),
        };
        if let Err(e) = parsed { eprintln!("{e}"); return usage(); }
    }
    let Some(dev) = dev else { return usage() };
    let result = match cmd.as_str() {
        "tap" if !points.is_empty() => tap(&dev, &points, Duration::from_millis(hold_ms), Duration::from_millis(gap_ms)),
        "record" => match (seconds, out) {
            (Some(s), Some(o)) => record(&dev, s, &o),
            _ => return usage(),
        },
        _ => return usage(),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => { eprintln!("potato-agent: {e}"); ExitCode::FAILURE }
    }
}
