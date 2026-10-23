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
//! `cdp` evaluates one JavaScript expression in a Chromium page through its DevTools port, on
//! loopback, and prints the reply. The kit aims taps from it: where an element is on the page, and
//! whether a tap at its centre reaches it. Afterwards it asks the app what state it is in.
//!
//! `keys` finds the keys of an on-screen keyboard in the framebuffer, so typing is aimed at where
//! the keys are drawn now rather than where they were when somebody measured them.
//!
//! Standard library only, so it builds as a static aarch64-unknown-linux-musl binary.

use std::fs::{File, OpenOptions};
use std::io::{ErrorKind, Read, Seek, SeekFrom, Write};
use std::net::TcpStream;
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::AsRawFd;
use std::process::ExitCode;
use std::thread::sleep;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

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

/// One finger, put down at `from`, dragged to `to` in `steps` moves over `ms`, held still for a
/// moment so the page does not fling, then lifted. The scroll a person makes with a thumb.
fn swipe(dev_path: &str, from: (i32, i32), to: (i32, i32), ms: u64, steps: u32) -> Result<(), String> {
    let mut dev = OpenOptions::new().write(true).open(dev_path).map_err(|e| format!("open {dev_path}: {e}"))?;
    let steps = steps.max(1);
    frame(&mut dev, &[
        (EV_ABS, ABS_MT_SLOT, 0),
        (EV_ABS, ABS_MT_TOUCH_MAJOR, 30),
        (EV_ABS, ABS_MT_WIDTH_MAJOR, 30),
        (EV_ABS, ABS_MT_PRESSURE, 30),
        (EV_ABS, ABS_MT_POSITION_X, from.0),
        (EV_ABS, ABS_MT_POSITION_Y, from.1),
        (EV_ABS, ABS_MT_TRACKING_ID, 2000),
        (EV_KEY, BTN_TOUCH, 1),
    ]).map_err(|e| format!("swipe down: {e}"))?;
    for i in 1..=steps {
        sleep(Duration::from_millis(ms / steps as u64));
        let x = from.0 + (to.0 - from.0) * i as i32 / steps as i32;
        let y = from.1 + (to.1 - from.1) * i as i32 / steps as i32;
        frame(&mut dev, &[(EV_ABS, ABS_MT_SLOT, 0), (EV_ABS, ABS_MT_POSITION_X, x), (EV_ABS, ABS_MT_POSITION_Y, y)])
            .map_err(|e| format!("swipe move {i}: {e}"))?;
    }
    sleep(Duration::from_millis(250));
    frame(&mut dev, &[(EV_ABS, ABS_MT_SLOT, 0), (EV_ABS, ABS_MT_TRACKING_ID, -1), (EV_KEY, BTN_TOUCH, 0)])
        .map_err(|e| format!("swipe up: {e}"))?;
    println!("POTATO_SWIPE {} {} {} {}", from.0, from.1, to.0, to.1);
    Ok(())
}

// The C library std already links; ioctl's request is an int in musl and an unsigned long in glibc.
extern "C" {
    #[cfg(target_env = "musl")]
    fn ioctl(fd: i32, request: i32, ...) -> i32;
    #[cfg(not(target_env = "musl"))]
    fn ioctl(fd: i32, request: std::ffi::c_ulong, ...) -> i32;
}

/// Slot 0's current value of a multitouch axis (EVIOCGMTSLOTS), or None if the kernel will not say.
fn mt_slot0(dev: &File, code: u16) -> Option<i32> {
    let mut buf = [0i32; 17]; // the code, then up to 16 slots
    buf[0] = code as i32;
    let request = (2u32 << 30) | ((buf.len() as u32 * 4) << 16) | (0x45 << 8) | 0x0a;
    #[cfg(target_env = "musl")]
    let r = unsafe { ioctl(dev.as_raw_fd(), request as i32, buf.as_mut_ptr()) };
    #[cfg(not(target_env = "musl"))]
    let r = unsafe { ioctl(dev.as_raw_fd(), request as std::ffi::c_ulong, buf.as_mut_ptr()) };
    (r >= 0).then_some(buf[1])
}

fn record(dev_path: &str, seconds: u64, out_path: &str) -> Result<(), String> {
    let mut dev = OpenOptions::new().read(true).custom_flags(O_NONBLOCK).open(dev_path)
        .map_err(|e| format!("open {dev_path}: {e}"))?;
    let mut out = File::create(out_path).map_err(|e| format!("create {out_path}: {e}"))?;
    // The kernel drops a position that repeats the slot's last one, so a tap at the same X as the
    // tap before it records no X at all (seen on 2026-10-23, two back-button taps). Where the slot
    // already stands is written beside the recording; "unknown" when the kernel will not say.
    let start = match (mt_slot0(&dev, ABS_MT_POSITION_X), mt_slot0(&dev, ABS_MT_POSITION_Y)) {
        (Some(x), Some(y)) => format!("{x} {y}\n"),
        _ => "unknown\n".to_string(),
    };
    std::fs::write(format!("{out_path}.start"), &start).map_err(|e| format!("write {out_path}.start: {e}"))?;
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

// ---- cdp: one expression, evaluated in a page ------------------------------------------------------

fn http_get(port: u16, path: &str) -> Result<String, String> {
    let mut s = TcpStream::connect(("127.0.0.1", port)).map_err(|e| format!("connect 127.0.0.1:{port}: {e}"))?;
    s.set_read_timeout(Some(Duration::from_secs(15))).map_err(|e| e.to_string())?;
    write!(s, "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n").map_err(|e| e.to_string())?;
    // Chromium's DevTools server keeps the connection open whatever the request says, so the body
    // is read to its Content-Length rather than to the end of the stream.
    let (mut raw, mut buf) = (Vec::new(), [0u8; 8192]);
    loop {
        if let Some(end) = raw.windows(4).position(|w| w == b"\r\n\r\n") {
            let head = String::from_utf8_lossy(&raw[..end]).into_owned();
            if !head.starts_with("HTTP/1.1 200") {
                return Err(format!("{path}: {}", head.lines().next().unwrap_or("")));
            }
            let len: usize = head.lines().find_map(|l| l.to_ascii_lowercase().strip_prefix("content-length:").map(|v| v.trim().to_string()))
                .and_then(|v| v.parse().ok()).ok_or_else(|| format!("{path}: no Content-Length"))?;
            if raw.len() >= end + 4 + len {
                return Ok(String::from_utf8_lossy(&raw[end + 4..end + 4 + len]).into_owned());
            }
        }
        let n = s.read(&mut buf).map_err(|e| format!("read {path}: {e}"))?;
        if n == 0 { return Err(format!("{path}: the connection closed early")); }
        raw.extend_from_slice(&buf[..n]);
    }
}

/// The top-level objects of a JSON array, as text, scanning braces outside strings.
fn json_objects(text: &str) -> Vec<&str> {
    let (mut out, mut depth, mut start, mut in_str, mut esc) = (Vec::new(), 0usize, 0usize, false, false);
    for (i, c) in text.char_indices() {
        if in_str {
            if esc { esc = false } else if c == '\\' { esc = true } else if c == '"' { in_str = false }
            continue;
        }
        match c {
            '"' => in_str = true,
            '{' => { if depth == 0 { start = i } depth += 1 }
            '}' if depth > 0 => { depth -= 1; if depth == 0 { out.push(&text[start..=i]) } }
            _ => {}
        }
    }
    out
}

/// A string field of a flat JSON object, unescaped only as far as quotes and backslashes.
fn json_field(obj: &str, name: &str) -> Option<String> {
    let at = obj.find(&format!("\"{name}\""))? + name.len() + 2;
    let rest = obj[at..].trim_start().strip_prefix(':')?.trim_start().strip_prefix('"')?;
    let (mut out, mut esc) = (String::new(), false);
    for c in rest.chars() {
        if esc { out.push(c); esc = false } else if c == '\\' { esc = true } else if c == '"' { return Some(out) } else { out.push(c) }
    }
    None
}

fn json_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn ws_send(s: &mut TcpStream, payload: &[u8]) -> Result<(), String> {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(0x5eed);
    let mask = nanos.to_le_bytes();
    let mut f = vec![0x81u8];
    match payload.len() {
        n if n < 126 => f.push(0x80 | n as u8),
        n if n < 65536 => { f.push(0x80 | 126); f.extend_from_slice(&(n as u16).to_be_bytes()) }
        n => { f.push(0x80 | 127); f.extend_from_slice(&(n as u64).to_be_bytes()) }
    }
    f.extend_from_slice(&mask);
    f.extend(payload.iter().enumerate().map(|(i, b)| b ^ mask[i % 4]));
    s.write_all(&f).map_err(|e| format!("websocket write: {e}"))
}

fn ws_recv(s: &mut TcpStream) -> Result<String, String> {
    let mut msg = Vec::new();
    loop {
        let mut h = [0u8; 2];
        s.read_exact(&mut h).map_err(|e| format!("websocket read: {e}"))?;
        let (fin, op) = (h[0] & 0x80 != 0, h[0] & 0x0f);
        let mut len = (h[1] & 0x7f) as u64;
        if len == 126 {
            let mut b = [0u8; 2]; s.read_exact(&mut b).map_err(|e| e.to_string())?; len = u16::from_be_bytes(b) as u64;
        } else if len == 127 {
            let mut b = [0u8; 8]; s.read_exact(&mut b).map_err(|e| e.to_string())?; len = u64::from_be_bytes(b);
        }
        let mut mask = [0u8; 4];
        if h[1] & 0x80 != 0 { s.read_exact(&mut mask).map_err(|e| e.to_string())?; }
        if len > 64 << 20 { return Err(format!("websocket frame of {len} bytes")); }
        let mut p = vec![0u8; len as usize];
        s.read_exact(&mut p).map_err(|e| format!("websocket read: {e}"))?;
        if h[1] & 0x80 != 0 { for (i, b) in p.iter_mut().enumerate() { *b ^= mask[i % 4] } }
        match op {
            0x8 => return Err("the page closed the connection".into()),
            0x0 | 0x1 => { msg.extend_from_slice(&p); if fin { return Ok(String::from_utf8_lossy(&msg).into_owned()) } }
            _ => {}
        }
    }
}

/// Evaluate `expr` in the page (the first one, or the one with this title) and print the reply as
/// `POTATO_CDP <json>`. The expression may return a promise; its value comes back by value.
fn cdp(port: u16, title: Option<&str>, expr: &str) -> Result<(), String> {
    let list = http_get(port, "/json/list")?;
    let page = json_objects(&list).into_iter().find(|o| {
        json_field(o, "type").as_deref() == Some("page") && title.is_none_or(|t| json_field(o, "title").as_deref() == Some(t))
    }).ok_or_else(|| format!("no page{} behind port {port}", title.map(|t| format!(" titled {t:?}")).unwrap_or_default()))?;
    let ws = json_field(page, "webSocketDebuggerUrl").ok_or("the page has no debugger url (is another client attached?)")?;
    let path = ws.split_once(&format!("127.0.0.1:{port}")).map(|(_, p)| p.to_string()).ok_or_else(|| format!("unexpected debugger url {ws}"))?;
    let mut s = TcpStream::connect(("127.0.0.1", port)).map_err(|e| format!("connect: {e}"))?;
    s.set_read_timeout(Some(Duration::from_secs(30))).map_err(|e| e.to_string())?;
    // The key is any 16 bytes in base64 ("potato-agent-key"); the reply's accept value is not checked.
    write!(s, "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\
               Sec-WebSocket-Key: cG90YXRvLWFnZW50LWtleQ==\r\nSec-WebSocket-Version: 13\r\n\r\n").map_err(|e| e.to_string())?;
    let mut head = Vec::new();
    let mut b = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        s.read_exact(&mut b).map_err(|e| format!("handshake: {e}"))?;
        head.push(b[0]);
        if head.len() > 8192 { return Err("handshake reply too long".into()); }
    }
    let head = String::from_utf8_lossy(&head);
    if !head.starts_with("HTTP/1.1 101") {
        return Err(format!("websocket refused: {}", head.lines().next().unwrap_or("")));
    }
    let call = format!("{{\"id\":1,\"method\":\"Runtime.evaluate\",\"params\":{{\"expression\":{},\"returnByValue\":true,\"awaitPromise\":true}}}}", json_string(expr));
    ws_send(&mut s, call.as_bytes())?;
    loop {
        let m = ws_recv(&mut s)?;
        if m.find("\"id\":1") == Some(1) {
            println!("POTATO_CDP {m}");
            return Ok(());
        }
    }
}

// ---- keys: an on-screen keyboard, found in the framebuffer ----------------------------------------

/// Rows of key caps inside `rect`, and each key's extent along its row. A row is a band of lines
/// that are mostly cap-coloured (brighter than `threshold`); a key is a run of columns that stay
/// cap-coloured through the middle half of the band. Labels drawn on the caps are thin and do not
/// break a run. Prints `POTATO_KEYROW <y0> <y1> <x0>-<x1> ...`, top row first.
fn keys(fb_path: &str, stride: usize, rect: (usize, usize, usize, usize), threshold: u32) -> Result<(), String> {
    let (rx, ry, rw, rh) = rect;
    let mut fb = File::open(fb_path).map_err(|e| format!("open {fb_path}: {e}"))?;
    fb.seek(SeekFrom::Start((ry * stride) as u64)).map_err(|e| e.to_string())?;
    let mut buf = vec![0u8; rh * stride];
    fb.read_exact(&mut buf).map_err(|e| format!("read {fb_path}: {e}"))?;
    if rx + rw > stride / 4 { return Err("the rectangle runs past the framebuffer's width".into()); }
    let lit = |x: usize, y: usize| -> bool {
        let i = y * stride + (rx + x) * 4;
        (buf[i] as u32 * 299 + buf[i + 1] as u32 * 587 + buf[i + 2] as u32 * 114) / 1000 > threshold
    };
    let mut bands = Vec::new();
    let mut open: Option<usize> = None;
    for y in 0..=rh {
        let on = y < rh && (0..rw).step_by(2).filter(|&x| lit(x, y)).count() * 2 > rw / 2;
        match (on, open) {
            (true, None) => open = Some(y),
            (false, Some(a)) => { if y - a >= 10 { bands.push((a, y)) } open = None }
            _ => {}
        }
    }
    for (a, b) in bands {
        let (m0, m1) = (a + (b - a) / 4, b - (b - a) / 4);
        let need = (m1 - m0) * 4 / 5;
        let mut runs = Vec::new();
        let mut start: Option<usize> = None;
        for x in 0..=rw {
            let on = x < rw && (m0..m1).filter(|&y| lit(x, y)).count() > need;
            match (on, start) {
                (true, None) => start = Some(x),
                (false, Some(s)) => { if x - s > 8 { runs.push(format!("{}-{}", rx + s, rx + x)) } start = None }
                _ => {}
            }
        }
        println!("POTATO_KEYROW {} {} {}", ry + a, ry + b, runs.join(" "));
    }
    Ok(())
}

fn parse_point(s: &str) -> Result<(i32, i32), String> {
    let (x, y) = s.split_once(',').ok_or_else(|| format!("expected X,Y, got {s}"))?;
    Ok((x.trim().parse().map_err(|_| format!("bad x in {s}"))?, y.trim().parse().map_err(|_| format!("bad y in {s}"))?))
}

fn parse_rect(s: &str) -> Result<(usize, usize, usize, usize), String> {
    let v: Vec<usize> = s.split(',').map(|p| p.trim().parse().map_err(|_| format!("bad rectangle {s}"))).collect::<Result<_, _>>()?;
    match v[..] { [x, y, w, h] if w > 0 && h > 0 => Ok((x, y, w, h)), _ => Err(format!("expected X,Y,W,H, got {s}")) }
}

fn usage() -> ExitCode {
    eprintln!("usage:\n  potato-agent tap --dev DEV --at X,Y [--at X,Y ...] [--hold-ms N] [--gap-ms N]\n  \
               potato-agent swipe --dev DEV --from X,Y --to X,Y [--ms N] [--steps N]\n  \
               potato-agent record --dev DEV --seconds N --out FILE\n  \
               potato-agent cdp --port N --expr-file FILE [--title TITLE]\n  \
               potato-agent keys --fb DEV --stride N --rect X,Y,W,H [--threshold N]");
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(cmd) = args.first().cloned() else { return usage() };
    let (mut dev, mut points, mut hold_ms, mut gap_ms, mut seconds, mut out) = (None, Vec::new(), 80u64, 300u64, None, None);
    let (mut port, mut expr_file, mut title, mut fb, mut stride, mut rect, mut threshold) = (None, None, None, None, None, None, 25u32);
    let (mut from, mut to, mut ms, mut steps) = (None, None, 500u64, 15u32);
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
            "--from" => parse_point(val).map(|p| from = Some(p)),
            "--to" => parse_point(val).map(|p| to = Some(p)),
            "--ms" => val.parse().map(|v| ms = v).map_err(|_| "bad --ms".into()),
            "--steps" => val.parse().map(|v| steps = v).map_err(|_| "bad --steps".into()),
            "--port" => val.parse().map(|v: u16| port = Some(v)).map_err(|_| "bad --port".into()),
            "--expr-file" => { expr_file = Some(val.clone()); Ok(()) }
            "--title" => { title = Some(val.clone()); Ok(()) }
            "--fb" => { fb = Some(val.clone()); Ok(()) }
            "--stride" => val.parse().map(|v: usize| stride = Some(v)).map_err(|_| "bad --stride".into()),
            "--rect" => parse_rect(val).map(|r| rect = Some(r)),
            "--threshold" => val.parse().map(|v| threshold = v).map_err(|_| "bad --threshold".into()),
            _ => return usage(),
        };
        if let Err(e) = parsed { eprintln!("{e}"); return usage(); }
    }
    let result = match (cmd.as_str(), &dev) {
        ("tap", Some(d)) if !points.is_empty() => tap(d, &points, Duration::from_millis(hold_ms), Duration::from_millis(gap_ms)),
        ("swipe", Some(d)) => match (from, to) {
            (Some(f), Some(t)) => swipe(d, f, t, ms, steps),
            _ => return usage(),
        },
        ("record", Some(d)) => match (seconds, out) {
            (Some(s), Some(o)) => record(d, s, &o),
            _ => return usage(),
        },
        ("cdp", _) => match (port, expr_file) {
            (Some(p), Some(f)) => std::fs::read_to_string(&f).map_err(|e| format!("read {f}: {e}")).and_then(|e| cdp(p, title.as_deref(), &e)),
            _ => return usage(),
        },
        ("keys", _) => match (fb, stride, rect) {
            (Some(f), Some(s), Some(r)) => keys(&f, s, r, threshold),
            _ => return usage(),
        },
        _ => return usage(),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => { eprintln!("potato-agent: {e}"); ExitCode::FAILURE }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_page_is_found_by_type_and_title_in_a_devtools_list() {
        let list = r#"[ {
   "title": "Omnibox Popup", "type": "browser_ui",
   "webSocketDebuggerUrl": "ws://127.0.0.1:9222/devtools/page/AAA"
}, {
   "title": "Orthodox Lampada", "type": "page", "url": "http://127.0.0.1:8973/{x}",
   "webSocketDebuggerUrl": "ws://127.0.0.1:9222/devtools/page/BBB"
} ]"#;
        let objs = json_objects(list);
        assert_eq!(objs.len(), 2, "a brace inside a string must not split an object");
        let page = objs.iter().find(|o| json_field(o, "type").as_deref() == Some("page")).unwrap();
        assert_eq!(json_field(page, "title").as_deref(), Some("Orthodox Lampada"));
        assert_eq!(json_field(page, "webSocketDebuggerUrl").as_deref(), Some("ws://127.0.0.1:9222/devtools/page/BBB"));
    }

    #[test]
    fn an_expression_survives_json_quoting() {
        assert_eq!(json_string("a \"b\"\n\\c"), r#""a \"b\"\n\\c""#);
    }
}
