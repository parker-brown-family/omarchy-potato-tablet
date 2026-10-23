//! potato: runs a diagnostic task against a tablet and writes the evidence and an unjudged record.
//!
//!     potato run <task.toml> --profile <profile.toml> --agent <potato-agent> --out <run-dir>
//!
//! The link is the tablet's USB serial gadget, found by vendor, product and serial in sysfs. Each
//! command goes over as a base64 script and its output comes back between sentinel lines that are
//! unique to that command, the protocol the onn bring-up's native-exec.py established.
//! Judging is not done here: every observer is written with `answer: null`.

use base64::{engine::general_purpose::STANDARD as B64, Engine};
use potato_record::{now_utc, Aim, Input, Observation, Record, TaskKind, TaskRun, FORMAT};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

type Res<T> = Result<T, String>;

// ---- profile and task files ---------------------------------------------------------------------

#[derive(Deserialize)]
struct Profile {
    tablet: TabletSection,
    link: LinkSection,
    touch: TouchSection,
    framebuffer: FbSection,
    calibration: CalibrationSection,
    desktop: DesktopSection,
    webcam: Option<WebcamSection>,
}
#[derive(Deserialize)]
struct TabletSection { name: String }
#[derive(Deserialize)]
struct LinkSection { usb_vendor: String, usb_product: String, usb_serial: String }
#[derive(Deserialize)]
struct TouchSection { device: String, x_max: i32, y_max: i32 }
#[derive(Deserialize)]
struct FbSection { device: String, height: u32, stride: u32 }
#[derive(Deserialize)]
struct CalibrationSection { matrix: Vec<f64>, source: String }
#[derive(Deserialize)]
struct DesktopSection { chroot: String, runtime_dir: String, home: String }
#[derive(Deserialize)]
struct WebcamSection { device: String, size: String, filter: String }

#[derive(Deserialize)]
struct Task {
    name: String,
    title: String,
    kind: TaskKind,
    input: Input,
    aim: TaskAim,
    expect: Expect,
}
#[derive(Deserialize)]
struct TaskAim { source: String, provenance: String, points: Vec<[i32; 2]>, hold_ms: u64, gap_ms: u64 }
#[derive(Deserialize)]
struct Expect { sink_text: String, taps: usize, framebuffer: String, webcam: String }

fn read_toml<T: for<'de> Deserialize<'de>>(path: &Path) -> Res<T> {
    let text = fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    toml::from_str(&text).map_err(|e| format!("parse {}: {e}", path.display()))
}

// ---- the serial link ----------------------------------------------------------------------------

fn find_port(vendor: &str, product: &str, serial: &str) -> Res<PathBuf> {
    let mut found = Vec::new();
    for entry in fs::read_dir("/sys/class/tty").map_err(|e| format!("/sys/class/tty: {e}"))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with("ttyACM") {
            continue;
        }
        let usb = entry.path().join("device/..");
        let attr = |a: &str| fs::read_to_string(usb.join(a)).map(|s| s.trim().to_string()).unwrap_or_default();
        if attr("idVendor") == vendor && attr("idProduct") == product && attr("serial") == serial {
            found.push(PathBuf::from("/dev").join(name));
        }
    }
    match found.len() {
        1 => Ok(found.remove(0)),
        0 => Err(format!("no tablet gadget {vendor}:{product} with serial {serial} is connected")),
        n => Err(format!("{n} gadgets match {vendor}:{product} {serial}; expected exactly one")),
    }
}

struct Link {
    port: File,
    seen: Arc<Mutex<Vec<u8>>>,
    log: File,
    next: u32,
    /// Where scripts are written on the other end: tablet RAM, or /tmp for the self-test.
    workdir: String,
}

impl Link {
    fn open(path: &Path, log_path: &Path) -> Res<Link> {
        let ok = Command::new("stty").args(["-F"]).arg(path).args(["raw", "-echo", "115200"]).status()
            .map_err(|e| format!("stty: {e}"))?;
        if !ok.success() {
            return Err(format!("stty could not configure {}", path.display()));
        }
        let port = OpenOptions::new().read(true).write(true).open(path)
            .map_err(|e| format!("open {}: {e} (is the uaccess udev rule installed?)", path.display()))?;
        let seen = Arc::new(Mutex::new(Vec::new()));
        let mut reader = port.try_clone().map_err(|e| e.to_string())?;
        let sink = Arc::clone(&seen);
        thread::spawn(move || {
            let mut buf = [0u8; 16384];
            while let Ok(n) = reader.read(&mut buf) {
                if n == 0 { thread::sleep(Duration::from_millis(5)); continue; }
                sink.lock().unwrap().extend_from_slice(&buf[..n]);
            }
        });
        let log = File::create(log_path).map_err(|e| format!("create {}: {e}", log_path.display()))?;
        let mut link = Link { port, seen, log, next: 1, workdir: "/run/potato".into() };
        // Quiet the tablet shell's echo so uploads do not come back over the cable.
        link.send(b"\nstty -echo 2>/dev/null\n")?;
        thread::sleep(Duration::from_millis(300));
        link.drain();
        Ok(link)
    }

    fn send(&mut self, bytes: &[u8]) -> Res<()> {
        self.port.write_all(bytes).map_err(|e| format!("serial write: {e}"))
    }

    fn drain(&mut self) -> Vec<u8> {
        let taken = std::mem::take(&mut *self.seen.lock().unwrap());
        let _ = self.log.write_all(&taken);
        taken
    }

    /// Write a file into the tablet's RAM through a base64 heredoc.
    fn put(&mut self, bytes: &[u8], remote: &str) -> Res<()> {
        if !remote.starts_with("/run/") && !remote.starts_with("/tmp/") {
            return Err(format!("uploads stay in tablet RAM directories, not {remote}"));
        }
        let parent = Path::new(remote).parent().map(|p| p.display().to_string()).unwrap_or_else(|| "/run".into());
        self.send(format!("\nmkdir -p {parent}; base64 -d >{remote} <<'POTATO_EOF'\n").as_bytes())?;
        let encoded = B64.encode(bytes);
        for (i, line) in encoded.as_bytes().chunks(76).enumerate() {
            self.send(line)?;
            self.send(b"\n")?;
            if i % 40 == 39 {
                thread::sleep(Duration::from_millis(10));
                self.drain();
            }
        }
        self.send(b"POTATO_EOF\n")
    }

    /// Run a script on the tablet and return (output, exit status).
    fn exec(&mut self, script: &str, timeout: Duration) -> Res<(String, i32)> {
        let n = self.next;
        self.next += 1;
        let path = format!("{}/s{n}.sh", self.workdir);
        self.put(script.as_bytes(), &path)?;
        self.drain();
        self.send(format!("printf '\\nPOTATO_BEGIN_{n}\\n'; /bin/sh {path}; printf '\\nPOTATO_STATUS_{n}=%s\\nPOTATO_END_{n}\\n' $?\n").as_bytes())?;
        let (begin, status, end) = (format!("POTATO_BEGIN_{n}"), format!("POTATO_STATUS_{n}="), format!("POTATO_END_{n}"));
        let deadline = Instant::now() + timeout;
        let end_line = format!("\n{end}");
        let mut got: Vec<u8> = Vec::new();
        loop {
            let fresh = self.drain();
            // Only the newly arrived bytes, plus a marker's width of overlap, can hold the end line.
            let from = got.len().saturating_sub(end_line.len() + 1);
            got.extend(fresh);
            if contains(&got[from..], end_line.as_bytes()) {
                let text = String::from_utf8_lossy(&got).replace('\r', "");
                let after_begin = text.split_once(&format!("{begin}\n")).map(|(_, rest)| rest).unwrap_or("");
                let (body, tail) = after_begin.split_once(&status).ok_or("status line missing")?;
                let code = tail.lines().next().unwrap_or("").trim().parse().unwrap_or(-1);
                return Ok((body.trim_end_matches('\n').to_string(), code));
            }
            if Instant::now() > deadline {
                return Err(format!("command {n} did not finish within {}s", timeout.as_secs()));
            }
            thread::sleep(Duration::from_millis(20));
        }
    }

    /// Fetch a file from the tablet.
    fn get(&mut self, remote: &str, timeout: Duration) -> Res<Vec<u8>> {
        let (out, code) = self.exec(&format!("test -f {remote}\nprintf 'POTATO_BLOB\\n'\nbase64 {remote}\n"), timeout)?;
        if code != 0 {
            return Err(format!("{remote} is not on the tablet"));
        }
        let b64: String = out.split_once("POTATO_BLOB\n").map(|(_, r)| r).unwrap_or("").split_whitespace().collect();
        B64.decode(b64).map_err(|e| format!("decode {remote}: {e}"))
    }
}

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

fn sha256(bytes: &[u8], scratch: &Path) -> Res<String> {
    let tmp = scratch.join(".sha-input");
    fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
    let out = Command::new("sha256sum").arg(&tmp).output().map_err(|e| format!("sha256sum: {e}"))?;
    let _ = fs::remove_file(&tmp);
    Ok(String::from_utf8_lossy(&out.stdout).split_whitespace().next().unwrap_or("").to_string())
}

// ---- captures -----------------------------------------------------------------------------------

fn capture_framebuffer(link: &mut Link, p: &Profile, out: &Path) -> Res<()> {
    let script = format!(
        "printf 'POTATO_FRAME\\n'\ndd if={} bs={} count={} 2>/dev/null | gzip -1 | base64\n",
        p.framebuffer.device, p.framebuffer.stride, p.framebuffer.height
    );
    let (text, code) = link.exec(&script, Duration::from_secs(120))?;
    if code != 0 {
        return Err(format!("framebuffer read failed with status {code}"));
    }
    let b64: String = text.split_once("POTATO_FRAME\n").map(|(_, r)| r).unwrap_or("").split_whitespace().collect();
    let gz = B64.decode(b64).map_err(|e| format!("framebuffer decode: {e}"))?;
    fs::write(out, gz).map_err(|e| e.to_string())
}

fn capture_webcam(p: &Profile, out: &Path) -> Res<()> {
    let w = p.webcam.as_ref().ok_or("this profile has no webcam")?;
    let vf = format!("select=gte(n\\,20),{}", w.filter);
    let ok = Command::new("timeout").args(["20", "ffmpeg", "-hide_banner", "-loglevel", "error", "-f", "v4l2", "-input_format", "mjpeg", "-video_size", &w.size, "-i", &w.device, "-vf", &vf, "-frames:v", "1", "-y"])
        .arg(out).status().map_err(|e| format!("ffmpeg: {e}"))?;
    if ok.success() && out.exists() { Ok(()) } else { Err(format!("no frame from {}", w.device)) }
}

// ---- the run ------------------------------------------------------------------------------------

fn desktop_env(p: &Profile) -> String {
    let d = &p.desktop;
    format!(
        "W=; for s in {rt}/wayland-*; do [ -S \"$s\" ] && W=${{s##*/}}; done; test -n \"$W\"\n\
         I=; for d in {rt}/hypr/*; do [ -d \"$d\" ] && I=${{d##*/}}; done; test -n \"$I\"\n\
         hypr() {{ chroot {root} /usr/bin/env XDG_RUNTIME_DIR={rt} HYPRLAND_INSTANCE_SIGNATURE=\"$I\" /usr/bin/hyprctl \"$@\"; }}\n",
        rt = d.runtime_dir, root = d.chroot
    ) + &format!("HOMEDIR={}\nROOT={}\nRT={}\n", d.home, d.chroot, d.runtime_dir)
}

/// The sink is the kit's own terminal, so its text is sized for a webcam to read: at the desktop's
/// 12-point font, a typed word through the C920 came back at Jev 0.46 (2026-10-23, first run).
fn sink_open_script(p: &Profile) -> String {
    desktop_env(p) + r#"set -eu
mkdir -p "$ROOT/tmp/potato"; : > "$ROOT/tmp/potato/typed"
setsid chroot "$ROOT" /usr/bin/env -i PATH=/usr/bin:/bin HOME="$HOMEDIR" LANG=C.UTF-8 XDG_RUNTIME_DIR="$RT" \
    XDG_CONFIG_HOME="$HOMEDIR/.config" WAYLAND_DISPLAY="$W" \
    /usr/bin/foot --title potato-sink --font=monospace:size=36 /bin/sh -c 'stty -icanon min 1; exec cat >> /tmp/potato/typed' \
    </dev/null >/tmp/potato-sink.log 2>&1 &
i=0
while [ $i -lt 40 ]; do
    hypr -j clients 2>/dev/null | grep -q '"title": "potato-sink"' && break
    sleep 1; i=$((i+1))
done
hypr -j clients 2>/dev/null | grep -q '"title": "potato-sink"' || { echo 'POTATO_SINK missing'; exit 3; }
sleep 2
echo "POTATO_SINK mapped after ${i}s"
hypr -j activewindow 2>/dev/null | grep -o '"title": "[^"]*"' | sed 's/^/POTATO_ACTIVE /'
echo POTATO_CLIENTS_BEGIN
hypr -j clients 2>/dev/null
echo POTATO_CLIENTS_END
"#
}

fn act_script(p: &Profile, aim: &TaskAim, seconds: u64) -> String {
    let ats: String = aim.points.iter().map(|[x, y]| format!(" --at {x},{y}")).collect();
    format!(
        "set -eu\nA=/run/potato/potato-agent\nrm -f /run/potato/events.bin\n\
         $A record --dev {dev} --seconds {seconds} --out /run/potato/events.bin >/run/potato/record.log 2>&1 &\nR=$!\nsleep 1\n\
         $A tap --dev {dev}{ats} --hold-ms {hold} --gap-ms {gap}\nwait $R\ncat /run/potato/record.log\n",
        dev = p.touch.device, hold = aim.hold_ms, gap = aim.gap_ms
    )
}

/// The sink window's rectangle from `hyprctl -j clients`, if Hyprland reported it.
fn sink_region(sink_out: &str) -> Option<[i32; 4]> {
    let json = sink_out.split_once("POTATO_CLIENTS_BEGIN\n")?.1.split_once("POTATO_CLIENTS_END")?.0;
    let clients: Vec<serde_json::Value> = serde_json::from_str(json.trim()).ok()?;
    let sink = clients.iter().find(|c| c["title"] == "potato-sink")?;
    let at = sink["at"].as_array()?;
    let size = sink["size"].as_array()?;
    Some([at.first()?.as_i64()? as i32, at.get(1)?.as_i64()? as i32, size.first()?.as_i64()? as i32, size.get(1)?.as_i64()? as i32])
}

fn observer(expected: &str, evidence: &[&str]) -> Observation {
    Observation { answer: None, expected: expected.into(), evidence: evidence.iter().map(|s| s.to_string()).collect(), detail: None, judged_by: None }
}

fn run(task_path: &Path, profile_path: &Path, agent_path: &Path, out: &Path) -> Res<()> {
    let p: Profile = read_toml(profile_path)?;
    let t: Task = read_toml(task_path)?;
    for [x, y] in &t.aim.points {
        if *x < 0 || *y < 0 || *x > p.touch.x_max || *y > p.touch.y_max {
            return Err(format!("aim point {x},{y} is outside the touchscreen's range"));
        }
    }
    fs::create_dir_all(out).map_err(|e| e.to_string())?;
    let started = now_utc();
    let port = find_port(&p.link.usb_vendor, &p.link.usb_product, &p.link.usb_serial)?;
    let mut link = Link::open(&port, &out.join("serial.log"))?;
    let mut notes = Vec::new();

    // The agent goes into RAM once per boot, verified by hash.
    let agent = fs::read(agent_path).map_err(|e| format!("read {}: {e}", agent_path.display()))?;
    let want = sha256(&agent, out)?;
    let (have, _) = link.exec("sha256sum /run/potato/potato-agent 2>/dev/null || true\n", Duration::from_secs(20))?;
    if !have.starts_with(&want) {
        link.put(&agent, "/run/potato/potato-agent")?;
        let (check, _) = link.exec("chmod 755 /run/potato/potato-agent\nsha256sum /run/potato/potato-agent\n", Duration::from_secs(60))?;
        if !check.contains(&want) {
            return Err(format!("agent upload did not verify: {check}"));
        }
        notes.push("agent uploaded and verified".to_string());
    }

    let evidence = |name: &str, r: Res<()>, notes: &mut Vec<String>| -> Option<String> {
        match r { Ok(()) => Some(name.to_string()), Err(e) => { notes.push(format!("{name}: {e}")); None } }
    };
    // The sink opens first, so "before" is the empty sink: the only change the act can cause is
    // the typed text. A "before" taken earlier would include the new window and the re-tiled
    // terminal behind it, which still shows the hello typed on 2026-10-22.
    let (sink_out, sink_code) = link.exec(&sink_open_script(&p), Duration::from_secs(90))?;
    let sink_ok = sink_code == 0;
    let region = sink_region(&sink_out);
    notes.push(format!("sink: {}", sink_out.lines().filter(|l| l.starts_with("POTATO_")).collect::<Vec<_>>().join("; ")));
    let fb_before = evidence("framebuffer-before.rgba.gz", capture_framebuffer(&mut link, &p, &out.join("framebuffer-before.rgba.gz")), &mut notes);
    let cam_before = evidence("webcam-before.jpg", capture_webcam(&p, &out.join("webcam-before.jpg")), &mut notes);

    let (acted, act_detail) = if sink_ok {
        let seconds = 3 + (t.aim.points.len() as u64 * (t.aim.hold_ms + t.aim.gap_ms)).div_ceil(1000) + 2;
        let (act_out, act_code) = link.exec(&act_script(&p, &t.aim, seconds), Duration::from_secs(seconds + 30))?;
        (Some(act_code == 0), act_out.lines().filter(|l| l.starts_with("POTATO_")).collect::<Vec<_>>().join("; "))
    } else {
        (Some(false), "not run: the sink window never mapped, so taps would have gone to whatever had focus".to_string())
    };
    thread::sleep(Duration::from_secs(2));

    let fb_after = evidence("framebuffer-after.rgba.gz", capture_framebuffer(&mut link, &p, &out.join("framebuffer-after.rgba.gz")), &mut notes);
    let cam_after = evidence("webcam-after.jpg", capture_webcam(&p, &out.join("webcam-after.jpg")), &mut notes);
    let typed = evidence("typed.txt", link.get(&format!("{}/tmp/potato/typed", p.desktop.chroot), Duration::from_secs(30))
        .and_then(|b| fs::write(out.join("typed.txt"), b).map_err(|e| e.to_string())), &mut notes);
    let events = if acted == Some(true) {
        evidence("events.bin", link.get("/run/potato/events.bin", Duration::from_secs(60))
            .and_then(|b| fs::write(out.join("events.bin"), b).map_err(|e| e.to_string())), &mut notes)
    } else { None };
    let _ = link.exec("pkill -f 'title potato-sink' || true\n", Duration::from_secs(20));
    let _ = link.send(b"stty echo 2>/dev/null\n");
    link.drain();

    let some = |v: &[&Option<String>]| -> Vec<String> { v.iter().filter_map(|o| (*o).clone()).collect() };
    let mut observers = BTreeMap::new();
    let ev = some(&[&events]);
    let sk = some(&[&typed]);
    let fb = some(&[&fb_before, &fb_after]);
    let cam = some(&[&cam_before, &cam_after]);
    observers.insert("input_events".into(), observer(&format!("{} touch-downs, each at its aimed point", t.expect.taps), &ev.iter().map(|s| s.as_str()).collect::<Vec<_>>()));
    observers.insert("sink".into(), observer(&format!("the sink received exactly \"{}\"", t.expect.sink_text), &sk.iter().map(|s| s.as_str()).collect::<Vec<_>>()));
    observers.insert("framebuffer".into(), observer(&t.expect.framebuffer, &fb.iter().map(|s| s.as_str()).collect::<Vec<_>>()));
    observers.insert("webcam".into(), observer(&t.expect.webcam, &cam.iter().map(|s| s.as_str()).collect::<Vec<_>>()));

    let m = &p.calibration.matrix;
    let record = Record {
        format: FORMAT,
        run: out.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(),
        tablet: p.tablet.name.clone(),
        link: format!("USB serial {}", port.display()),
        started: started.clone(),
        tasks: vec![TaskRun {
            task: t.name.clone(),
            title: t.title.clone(),
            kind: t.kind,
            input: t.input,
            aim: Some(Aim { source: t.aim.source.clone(), provenance: t.aim.provenance.clone(), points: t.aim.points.clone() }),
            calibration: Some(format!("matrix {m:?}, from {}", p.calibration.source)),
            acted,
            act_detail: Some([act_detail, notes.join("; ")].join(" | ")),
            region,
            started,
            finished: Some(now_utc()),
            observers,
        }],
    };
    let json = serde_json::to_string_pretty(&record).map_err(|e| e.to_string())?;
    fs::write(out.join("record.json"), json + "\n").map_err(|e| e.to_string())?;
    println!("record: {}", out.join("record.json").display());
    Ok(())
}

/// Exercise the link's protocol against any shell on a serial-like port: a script and its status,
/// a large upload and download, and the round trip's hash. Used against the tablet's own BusyBox
/// under qemu on a pseudo-terminal, so the protocol is proven before it touches the tablet.
fn selftest(port: &Path, out: &Path) -> Res<()> {
    fs::create_dir_all(out).map_err(|e| e.to_string())?;
    let mut link = Link::open(port, &out.join("serial.log"))?;
    link.workdir = "/tmp/potato-selftest".into();
    let (text, code) = link.exec("echo first line\necho second line\nexit 7\n", Duration::from_secs(20))?;
    if text != "first line\nsecond line" || code != 7 {
        return Err(format!("exec returned {text:?} with status {code}"));
    }
    println!("exec: output and exit status 7 came back intact");
    let blob: Vec<u8> = (0..300_000u32).map(|i| (i.wrapping_mul(2_654_435_761) >> 13) as u8).collect();
    link.put(&blob, "/tmp/potato-selftest/blob.bin")?;
    let back = link.get("/tmp/potato-selftest/blob.bin", Duration::from_secs(120))?;
    if back != blob {
        return Err(format!("round trip changed the blob: sent {} bytes, got {}", blob.len(), back.len()));
    }
    let (remote_hash, _) = link.exec("sha256sum /tmp/potato-selftest/blob.bin\n", Duration::from_secs(20))?;
    if !remote_hash.starts_with(&sha256(&blob, out)?) {
        return Err(format!("remote hash differs: {remote_hash}"));
    }
    println!("put and get: {} bytes each way, hashes match", blob.len());
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let flag = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).map(PathBuf::from);
    let result = match (args.first().map(String::as_str), args.get(1), flag("--profile"), flag("--agent"), flag("--out")) {
        (Some("run"), Some(task), Some(profile), Some(agent), Some(out)) => run(Path::new(task), &profile, &agent, &out),
        (Some("selftest"), _, _, _, Some(out)) => match flag("--port") {
            Some(port) => selftest(&port, &out),
            None => Err("selftest needs --port".into()),
        },
        _ => {
            eprintln!("usage: potato run <task.toml> --profile <profile.toml> --agent <potato-agent> --out <run-dir>\n       potato selftest --port <tty> --out <dir>");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => { eprintln!("potato: {e}"); ExitCode::FAILURE }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLIENTS: &str = "noise\nPOTATO_CLIENTS_BEGIN\n[{\"title\": \"onn-omarchy:/#\", \"at\": [0, 0], \"size\": [1200, 760]},\n {\"title\": \"potato-sink\", \"at\": [0, 770], \"size\": [1200, 760], \"mapped\": true}]\nPOTATO_CLIENTS_END\nmore";

    #[test]
    fn the_sink_region_is_the_window_titled_potato_sink() {
        assert_eq!(sink_region(CLIENTS), Some([0, 770, 1200, 760]));
    }

    #[test]
    fn no_sink_window_means_no_region() {
        assert_eq!(sink_region(&CLIENTS.replace("potato-sink", "something-else")), None);
        assert_eq!(sink_region("POTATO_CLIENTS_BEGIN\nnot json\nPOTATO_CLIENTS_END"), None);
    }

    #[test]
    fn the_act_script_taps_every_aimed_point_in_order() {
        let aim = TaskAim { source: "replay".into(), provenance: "test".into(), points: vec![[104, 998], [144, 563]], hold_ms: 80, gap_ms: 350 };
        let p: Profile = toml::from_str(include_str!("../../../profiles/onn-10-pro.toml")).unwrap();
        let s = act_script(&p, &aim, 9);
        assert!(s.contains("tap --dev /dev/input/event3 --at 104,998 --at 144,563 --hold-ms 80 --gap-ms 350"), "{s}");
        assert!(s.contains("record --dev /dev/input/event3 --seconds 9"), "{s}");
    }
}
