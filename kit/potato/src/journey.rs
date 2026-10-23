//! A journey: several steps inside one app, each a synthetic touch and the outcome it should cause.
//!
//!     potato journey <journey.toml> --profile <profile.toml> --agent <potato-agent> --out <run-dir> [--film]
//!
//! Every tap is aimed from the live layout, never from recorded points. In a web app, the page says
//! where the element is (Chromium's DevTools port, through `potato-agent cdp`), and the window and
//! the monitor's scale carry that onto the screen. On the on-screen keyboard, the keys are found in
//! the framebuffer as it is drawn now (`potato-agent keys`), and labelled by the profile's layout.
//! The calibration then carries each screen point into the touchscreen's own coordinates.
//!
//! After the act, the app is asked whether the intended outcome happened (an exact check), and the
//! framebuffer and webcam are captured for the judge. With `--film`, the webcam records the whole
//! walk and its stills are cut from the film at the moments they were taken.

use super::*;
use std::process::{Child, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Deserialize)]
pub(crate) struct Journey {
    name: String,
    title: String,
    kind: TaskKind,
    input: Input,
    app: App,
    step: Vec<Step>,
}

#[derive(Deserialize)]
struct App {
    /// The page title, which is also the window title Hyprland reports.
    title: String,
    url: String,
    /// A command inside the desktop's root that opens the app: `<launch> <url> <title>`.
    launch: String,
    cdp_port: u16,
    /// Run once before the first step, to put the app on its first screen. Setup, not a test.
    start: String,
    start_check: String,
    start_settle_ms: u64,
    settle_ms: u64,
    hold_ms: u64,
    gap_ms: u64,
}

#[derive(Deserialize)]
struct Step {
    name: String,
    title: String,
    /// A CSS selector for the element to tap; with `text`, the first visible one containing it.
    tap: Option<String>,
    text: Option<String>,
    /// Typed on the on-screen keyboard, after the tap (if any) has focused a field.
    keys: Option<String>,
    /// Instead of tapping the element, drag from its centre by this many CSS pixels: a scroll.
    swipe: Option<[f64; 2]>,
    /// What should happen, in words, and the JavaScript expression that is true when it has.
    outcome: String,
    check: String,
    /// What the screen should show afterwards and not before: read by Jev in both pictures.
    screen: String,
    settle_ms: Option<u64>,
}

#[derive(Deserialize)]
pub(crate) struct KeyboardSection {
    /// The layer-shell namespace the keyboard's surface has in `hyprctl layers`.
    layer: String,
    threshold: u32,
    /// The layout's labels, top row first, left to right. `space` names the space bar.
    rows: Vec<Vec<String>>,
}

// ---- geometry -----------------------------------------------------------------------------------

fn section<'a>(out: &'a str, name: &str) -> Option<&'a str> {
    Some(out.split_once(&format!("POTATO_{name}_BEGIN\n"))?.1.split_once(&format!("POTATO_{name}_END"))?.0.trim())
}

struct Screen {
    /// The app window, logical pixels: x, y, w, h.
    window: Option<[f64; 4]>,
    scale: f64,
    /// The keyboard's layer, logical pixels.
    keyboard: Option<[f64; 4]>,
}

fn geometry_script(p: &Profile) -> String {
    desktop_env(p) + "echo POTATO_CLIENTS_BEGIN; hypr -j clients; echo POTATO_CLIENTS_END\n\
        echo POTATO_MONITORS_BEGIN; hypr -j monitors; echo POTATO_MONITORS_END\n\
        echo POTATO_LAYERS_BEGIN; hypr -j layers; echo POTATO_LAYERS_END\n"
}

fn parse_screen(out: &str, title: &str, layer: Option<&str>) -> Res<Screen> {
    let num = |v: &serde_json::Value| v.as_f64().unwrap_or(f64::NAN);
    let clients: Vec<serde_json::Value> = serde_json::from_str(section(out, "CLIENTS").ok_or("no client list")?).map_err(|e| format!("clients: {e}"))?;
    let window = clients.iter().find(|c| c["title"] == title).map(|c| [num(&c["at"][0]), num(&c["at"][1]), num(&c["size"][0]), num(&c["size"][1])]);
    let monitors: Vec<serde_json::Value> = serde_json::from_str(section(out, "MONITORS").ok_or("no monitor list")?).map_err(|e| format!("monitors: {e}"))?;
    let scale = monitors.first().map(|m| num(&m["scale"])).filter(|s| s.is_finite() && *s > 0.0).ok_or("no monitor scale")?;
    let mut keyboard = None;
    if let (Some(layer), Some(json)) = (layer, section(out, "LAYERS")) {
        let layers: serde_json::Value = serde_json::from_str(json).map_err(|e| format!("layers: {e}"))?;
        for mon in layers.as_object().into_iter().flat_map(|m| m.values()) {
            for level in mon["levels"].as_object().into_iter().flat_map(|l| l.values()) {
                for l in level.as_array().into_iter().flatten() {
                    if l["namespace"] == layer {
                        keyboard = Some([num(&l["x"]), num(&l["y"]), num(&l["w"]), num(&l["h"])]);
                    }
                }
            }
        }
    }
    Ok(Screen { window, scale, keyboard })
}

/// A screen pixel into the touchscreen's coordinates, through the inverse of the calibration's
/// libinput matrix: screen = M · touch, both normalised to 0..1.
pub(crate) fn screen_to_touch(p: &Profile, sx: f64, sy: f64) -> Res<[i32; 2]> {
    let m = &p.calibration.matrix;
    if m.len() != 6 {
        return Err("the calibration matrix needs six numbers".into());
    }
    let (s, t) = (sx / p.framebuffer.width as f64, sy / p.framebuffer.height as f64);
    let det = m[0] * m[4] - m[1] * m[3];
    if det.abs() < 1e-9 {
        return Err("the calibration matrix cannot be inverted".into());
    }
    let u = (m[4] * (s - m[2]) - m[1] * (t - m[5])) / det;
    let v = (m[0] * (t - m[5]) - m[3] * (s - m[2])) / det;
    let (x, y) = ((u * p.touch.x_max as f64).round() as i32, (v * p.touch.y_max as f64).round() as i32);
    if x < 0 || y < 0 || x > p.touch.x_max || y > p.touch.y_max {
        return Err(format!("screen point {sx:.0},{sy:.0} falls outside the touchscreen ({x},{y})"));
    }
    Ok([x, y])
}

/// Where the element is, as the page reports it, and whether a tap at its centre reaches it.
fn aim_expression(selector: &str, text: Option<&str>) -> String {
    let sel = serde_json::to_string(selector).unwrap_or_default();
    let text = text.map(|t| serde_json::to_string(t).unwrap_or_default()).unwrap_or_else(|| "null".into());
    format!(r#"(() => {{
  const sel = {sel}, text = {text};
  const els = [...document.querySelectorAll(sel)].filter(e => text === null || e.textContent.includes(text));
  const vis = els.filter(e => {{ const r = e.getBoundingClientRect(); return r.width > 0 && r.height > 0; }});
  if (!vis.length) return {{matches: els.length, visible: 0}};
  const e = vis[0], r = e.getBoundingClientRect();
  const cx = r.left + r.width / 2, cy = r.top + r.height / 2;
  const hit = document.elementFromPoint(cx, cy);
  return {{matches: els.length, visible: vis.length, cx, cy, w: r.width, h: r.height, iw: innerWidth, ih: innerHeight,
          dpr: devicePixelRatio, on_page: cx >= 0 && cy >= 0 && cx < innerWidth && cy < innerHeight,
          reaches: !!hit && (hit === e || e.contains(hit)),
          label: (e.getAttribute('aria-label') || e.textContent).replace(/\s+/g, ' ').trim().slice(0, 60)}};
}})()"#)
}

/// The element's centre on the screen, in framebuffer pixels. The page is the bottom-right of its
/// window (an app window has no browser chrome, and any bar sits above the page), and a CSS pixel
/// is `dpr / scale` logical pixels.
fn element_on_screen(aim: &serde_json::Value, window: [f64; 4], scale: f64) -> Res<(f64, f64, String)> {
    let f = |k: &str| aim[k].as_f64().ok_or_else(|| format!("the page did not report {k}"));
    let label = aim["label"].as_str().unwrap_or("").to_string();
    if aim["visible"].as_u64() == Some(0) {
        return Err(format!("no visible element matches ({} in the page)", aim["matches"]));
    }
    if aim["on_page"] != true {
        return Err(format!("\"{label}\" is off the visible page; the kit does not scroll"));
    }
    if aim["reaches"] != true {
        return Err(format!("something covers the centre of \"{label}\""));
    }
    let k = f("dpr")? / scale;
    let (ox, oy) = (window[0] + window[2] - f("iw")? * k, window[1] + window[3] - f("ih")? * k);
    Ok(((ox + f("cx")? * k) * scale, (oy + f("cy")? * k) * scale, label))
}

/// Key centres for `text`, from the rows `potato-agent keys` found and the layout's labels.
pub(crate) fn key_points(found: &[(i32, i32, Vec<(i32, i32)>)], layout: &[Vec<String>], text: &str) -> Res<Vec<(i32, i32)>> {
    let counts: Vec<usize> = found.iter().map(|r| r.2.len()).collect();
    let want: Vec<usize> = layout.iter().map(|r| r.len()).collect();
    if counts != want {
        return Err(format!("the keyboard on screen has rows of {counts:?} keys; the profile's layout has {want:?}"));
    }
    text.chars().map(|c| {
        let label = if c == ' ' { "space".to_string() } else { c.to_string() };
        layout.iter().enumerate().find_map(|(r, row)| row.iter().position(|l| *l == label).map(|i| (r, i)))
            .map(|(r, i)| ((found[r].2[i].0 + found[r].2[i].1) / 2, (found[r].0 + found[r].1) / 2))
            .ok_or_else(|| format!("no key for {c:?} in the layout"))
    }).collect()
}

fn parse_keyrows(out: &str) -> Vec<(i32, i32, Vec<(i32, i32)>)> {
    out.lines().filter_map(|l| l.strip_prefix("POTATO_KEYROW ")).filter_map(|l| {
        let mut it = l.split_whitespace();
        let (y0, y1) = (it.next()?.parse().ok()?, it.next()?.parse().ok()?);
        let keys = it.filter_map(|k| k.split_once('-')).filter_map(|(a, b)| Some((a.parse().ok()?, b.parse().ok()?))).collect();
        Some((y0, y1, keys))
    }).collect()
}

// ---- the page ------------------------------------------------------------------------------------

fn cdp(link: &mut Link, port: u16, title: &str, expr: &str) -> Res<serde_json::Value> {
    if title.contains('\'') {
        return Err("an app title with a single quote cannot be passed to the agent".into());
    }
    link.put(expr.as_bytes(), "/run/potato/expr.js")?;
    let (out, code) = link.exec(&format!("/run/potato/potato-agent cdp --port {port} --title '{title}' --expr-file /run/potato/expr.js 2>&1\n"), Duration::from_secs(40))?;
    let line = out.lines().find_map(|l| l.strip_prefix("POTATO_CDP ")).ok_or_else(|| format!("no reply from the page (status {code}): {}", out.trim()))?;
    let reply: serde_json::Value = serde_json::from_str(line).map_err(|e| format!("reply: {e}"))?;
    if let Some(ex) = reply["result"].get("exceptionDetails") {
        return Err(format!("the page threw: {}", ex["exception"]["description"].as_str().or(ex["text"].as_str()).unwrap_or("?")));
    }
    Ok(reply["result"]["result"].get("value").cloned().unwrap_or(serde_json::Value::Null))
}

// ---- the film ------------------------------------------------------------------------------------

fn wallclock() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(f64::NAN)
}

/// The webcam, recording for the whole walk. Frames carry wall-clock timestamps, so a still taken
/// at a moment is cut from the film at that moment, and the camera is opened once.
struct Film { child: Child, path: PathBuf }

impl Film {
    fn start(w: &WebcamSection, path: &Path) -> Res<Film> {
        let child = Command::new("ffmpeg")
            .args(["-hide_banner", "-loglevel", "error", "-f", "v4l2", "-input_format", "mjpeg", "-video_size", &w.size,
                   "-use_wallclock_as_timestamps", "1", "-i", &w.device, "-c:v", "copy", "-copyts", "-y"])
            .arg(path).stdin(Stdio::piped()).stdout(Stdio::null()).spawn().map_err(|e| format!("ffmpeg: {e}"))?;
        thread::sleep(Duration::from_secs(2));
        Ok(Film { child, path: path.to_path_buf() })
    }

    fn stop(mut self) -> Res<f64> {
        if let Some(mut stdin) = self.child.stdin.take() {
            let _ = stdin.write_all(b"q");
        }
        let deadline = Instant::now() + Duration::from_secs(20);
        while self.child.try_wait().map_err(|e| e.to_string())?.is_none() {
            if Instant::now() > deadline { let _ = self.child.kill(); break; }
            thread::sleep(Duration::from_millis(100));
        }
        let out = Command::new("ffprobe").args(["-v", "error", "-show_entries", "format=start_time", "-of", "csv=p=0"])
            .arg(&self.path).output().map_err(|e| format!("ffprobe: {e}"))?;
        String::from_utf8_lossy(&out.stdout).trim().parse().map_err(|_| "the film has no start time".to_string())
    }
}

fn still_from_film(film: &Path, start: f64, at: f64, filter: &str, out: &Path) -> Res<()> {
    let ok = Command::new("ffmpeg").args(["-hide_banner", "-loglevel", "error", "-ss", &format!("{:.3}", at - start), "-i"])
        .arg(film).args(["-vf", filter, "-frames:v", "1", "-y"]).arg(out).status().map_err(|e| format!("ffmpeg: {e}"))?;
    if ok.success() && out.exists() { Ok(()) } else { Err(format!("no frame at {:.1} s into the film", at - start)) }
}

// ---- the walk ------------------------------------------------------------------------------------

enum Cam { Still, Filmed(f64) }

struct Done {
    run: TaskRun,
    dir: String,
    cams: Vec<(String, Cam)>,
    /// Wall-clock moment the act began, for the film's timeline.
    act_at: Option<f64>,
}

pub(crate) fn journey(journey_path: &Path, profile_path: &Path, agent_path: &Path, out: &Path, film: bool) -> Res<()> {
    let p: Profile = read_toml(profile_path)?;
    let j: Journey = read_toml(journey_path)?;
    let a = &j.app;
    fs::create_dir_all(out).map_err(|e| e.to_string())?;
    let started = now_utc();
    let port = find_port(&p.link.usb_vendor, &p.link.usb_product, &p.link.usb_serial)?;
    let mut link = Link::open(&port, &out.join("serial.log"))?;
    let mut notes = vec![ensure_agent(&mut link, agent_path, out)?];
    let layer = p.keyboard.as_ref().map(|k| k.layer.as_str());

    // The app: opened if its page is not there, then put on its first screen.
    if cdp(&mut link, a.cdp_port, &a.title, "document.title").is_err() {
        let (o, code) = link.exec(&(desktop_env(&p) + &format!("chroot \"$ROOT\" {} '{}' '{}' 2>&1\n", a.launch, a.url, a.title)), Duration::from_secs(90))?;
        notes.push(format!("launched the app (status {code}) {}", o.trim()));
        thread::sleep(Duration::from_secs(5));
    }
    let _ = cdp(&mut link, a.cdp_port, &a.title, &a.start);
    thread::sleep(Duration::from_millis(a.start_settle_ms));
    let at_start = cdp(&mut link, a.cdp_port, &a.title, &a.start_check);
    let mut stopped: Option<String> = match &at_start {
        Ok(v) if *v == serde_json::Value::Bool(true) => None,
        Ok(v) => Some(format!("the app was not on its first screen after setup ({} gave {v})", a.start_check)),
        Err(e) => Some(format!("the app could not be reached: {e}")),
    };
    notes.push(format!("setup: {} then {} -> {:?}", a.start, a.start_check, at_start));

    let filming = match (film, p.webcam.as_ref()) {
        (true, Some(w)) => Some(Film::start(w, &out.join("film.mkv"))?),
        _ => None,
    };
    let m = &p.calibration.matrix;
    let calibration = format!("matrix {m:?}, from {}", p.calibration.source);
    let mut done: Vec<Done> = Vec::new();

    for (i, s) in j.step.iter().enumerate() {
        let dir = format!("{:02}-{}", i + 1, s.name);
        fs::create_dir_all(out.join(&dir)).map_err(|e| e.to_string())?;
        let rel = |f: &str| format!("{dir}/{f}");
        let step_started = now_utc();
        let mut detail = Vec::new();
        let mut points: Vec<[i32; 2]> = Vec::new();
        let mut aimed_at = Vec::new();
        let mut swipe_to: Option<[i32; 2]> = None;
        let mut act_at = None;
        let mut region = None;
        let mut cams = Vec::new();

        // Aim from what is on the screen now.
        let aim: Res<()> = (|| {
            if let Some(why) = &stopped { return Err(why.clone()); }
            let (g, _) = link.exec(&geometry_script(&p), Duration::from_secs(30))?;
            let screen = parse_screen(&g, &a.title, layer)?;
            let win = screen.window.ok_or_else(|| format!("no window titled {:?}", a.title))?;
            region = Some([(win[0] * screen.scale) as i32, (win[1] * screen.scale) as i32, (win[2] * screen.scale) as i32, (win[3] * screen.scale) as i32]);
            if let Some(sel) = &s.tap {
                let v = cdp(&mut link, a.cdp_port, &a.title, &aim_expression(sel, s.text.as_deref()))?;
                let (sx, sy, label) = element_on_screen(&v, win, screen.scale)?;
                points.push(screen_to_touch(&p, sx, sy)?);
                aimed_at.push(format!("\"{label}\" at screen {sx:.0},{sy:.0}"));
                if let Some([dx, dy]) = s.swipe {
                    let k = v["dpr"].as_f64().unwrap_or(screen.scale);
                    let (ex, ey) = (sx + dx * k, sy + dy * k);
                    swipe_to = Some(screen_to_touch(&p, ex, ey)?);
                    aimed_at.push(format!("dragged to screen {ex:.0},{ey:.0}"));
                }
            }
            if let Some(text) = &s.keys {
                let kb = p.keyboard.as_ref().ok_or("this profile has no keyboard layout")?;
                let r = screen.keyboard.ok_or_else(|| format!("no {} layer on screen", kb.layer))?;
                let k = screen.scale;
                let rect = format!("{},{},{},{}", (r[0] * k) as i32, (r[1] * k) as i32, (r[2] * k) as i32, ((r[3] * k) as i32).min(p.framebuffer.height as i32 - (r[1] * k) as i32));
                let (o, _) = link.exec(&format!("/run/potato/potato-agent keys --fb {} --stride {} --rect {rect} --threshold {} 2>&1\n", p.framebuffer.device, p.framebuffer.stride, kb.threshold), Duration::from_secs(30))?;
                for (x, y) in key_points(&parse_keyrows(&o), &kb.rows, text)? {
                    points.push(screen_to_touch(&p, x as f64, y as f64)?);
                }
                aimed_at.push(format!("keys {text:?} found on the {} layer at {rect}", kb.layer));
            }
            if points.is_empty() { return Err("the step names nothing to tap".into()); }
            Ok(())
        })();

        let acted = match aim {
            Err(why) => { detail.push(format!("not run: {why}")); Some(false) }
            Ok(()) => {
                let capture = |link: &mut Link, when: &str, cams: &mut Vec<(String, Cam)>, detail: &mut Vec<String>| {
                    if let Err(e) = capture_framebuffer(link, &p, &out.join(rel(&format!("framebuffer-{when}.rgba.gz")))) {
                        detail.push(format!("framebuffer {when}: {e}"));
                    }
                    let name = rel(&format!("webcam-{when}.jpg"));
                    if filming.is_some() {
                        cams.push((name, Cam::Filmed(wallclock())));
                    } else if p.webcam.is_some() {
                        cams.push((name, Cam::Still));
                    }
                };
                capture(&mut link, "before", &mut cams, &mut detail);
                if filming.is_none() { still_now(&p, out, &cams, "before", &mut detail); }
                let seconds = 3 + (points.len() as u64 * (a.hold_ms + a.gap_ms)).div_ceil(1000) + 2;
                let script = match swipe_to {
                    Some(to) => swipe_script(&p, points[0], to, seconds),
                    None => act_script(&p, &points, a.hold_ms, a.gap_ms, seconds),
                };
                act_at = Some(wallclock() + 1.0); // the act script waits a second for the recorder
                let (o, code) = link.exec(&script, Duration::from_secs(seconds + 30))?;
                detail.push(format!("aimed {}; {}", aimed_at.join("; "), o.lines().filter(|l| l.starts_with("POTATO_")).collect::<Vec<_>>().join("; ")));
                thread::sleep(Duration::from_millis(s.settle_ms.unwrap_or(a.settle_ms)));
                capture(&mut link, "after", &mut cams, &mut detail);
                if filming.is_none() { still_now(&p, out, &cams, "after", &mut detail); }
                let check = cdp(&mut link, a.cdp_port, &a.title, &s.check);
                let state = match &check {
                    Ok(v) => serde_json::json!({ "expression": s.check, "value": v }),
                    Err(e) => serde_json::json!({ "expression": s.check, "error": e }),
                };
                let _ = fs::write(out.join(rel("app-state.json")), serde_json::to_string_pretty(&state).unwrap_or_default() + "\n");
                if !matches!(check, Ok(serde_json::Value::Bool(true))) {
                    stopped = Some(format!("the walk stopped after \"{}\": the app did not report its outcome", s.title));
                }
                for (remote, local) in [("/run/potato/events.bin", "events.bin"), ("/run/potato/events.bin.start", "events.start")] {
                    if let Err(e) = link.get(remote, Duration::from_secs(60)).and_then(|b| fs::write(out.join(rel(local)), b).map_err(|e| e.to_string())) {
                        detail.push(format!("{local}: {e}"));
                    }
                }
                Some(code == 0)
            }
        };

        let have = |f: &str| out.join(rel(f)).exists().then(|| rel(f));
        let list = |v: &[Option<String>]| v.iter().flatten().cloned().collect::<Vec<_>>();
        let mut observers = BTreeMap::new();
        let o = |expected: String, evidence: Vec<String>| Observation { answer: None, expected, evidence, detail: None, judged_by: None };
        observers.insert("input_events".into(), o(format!("{} touch-downs, each at its aimed point", points.len()), list(&[have("events.bin"), have("events.start")])));
        observers.insert("app_state".into(), o(format!("{} (the app reports `{}`)", s.outcome, s.check), list(&[have("app-state.json")])));
        observers.insert("framebuffer".into(), o(s.screen.clone(), list(&[have("framebuffer-before.rgba.gz"), have("framebuffer-after.rgba.gz")])));
        observers.insert("webcam".into(), o(s.screen.clone(), Vec::new()));
        let source = match (&s.tap, &s.keys, s.swipe) { (Some(_), _, Some(_)) => "dom, swipe", (Some(_), Some(_), _) => "dom + keyboard", (Some(_), None, _) => "dom", _ => "keyboard" };
        done.push(Done {
            run: TaskRun {
                task: format!("{}/{}", j.name, s.name),
                title: s.title.clone(),
                kind: j.kind,
                input: j.input,
                aim: Some(Aim { source: source.into(), provenance: if aimed_at.is_empty() { "not aimed".into() } else { aimed_at.join("; ") }, points: points.clone() }),
                calibration: Some(calibration.clone()),
                acted,
                act_detail: Some(detail.join(" | ")),
                region,
                started: step_started,
                finished: Some(now_utc()),
                observers,
            },
            dir,
            cams,
            act_at,
        });
        println!("{:02} {}: {}", i + 1, s.title, if acted == Some(true) && stopped.is_none() { "done" } else { stopped.as_deref().unwrap_or("not run") });
    }

    // The film's stills are cut once the camera has stopped.
    let film_start = match filming {
        Some(f) => { thread::sleep(Duration::from_secs(3)); Some(f.stop()?) }
        None => None,
    };
    for d in &mut done {
        for (name, cam) in &d.cams {
            let ok = match (cam, film_start, p.webcam.as_ref()) {
                (Cam::Filmed(at), Some(start), Some(w)) => still_from_film(&out.join("film.mkv"), start, *at, &w.filter, &out.join(name)).is_ok(),
                _ => out.join(name).exists(),
            };
            if ok {
                d.run.observers.get_mut("webcam").expect("inserted above").evidence.push(name.clone());
            }
        }
    }
    // The film's timeline, so an edit can find each step: seconds from the film's first frame.
    if let Some(start) = film_start {
        let at = |t: Option<f64>| t.map(|t| ((t - start) * 10.0).round() / 10.0);
        let steps: Vec<serde_json::Value> = done.iter().map(|d| {
            let cam = |w: &str| d.cams.iter().find(|(n, _)| n.ends_with(w)).and_then(|(_, c)| match c { Cam::Filmed(t) => Some(*t), Cam::Still => None });
            serde_json::json!({ "dir": d.dir, "title": d.run.title, "before": at(cam("webcam-before.jpg")), "act": at(d.act_at), "after": at(cam("webcam-after.jpg")) })
        }).collect();
        let timeline = serde_json::json!({ "film": "film.mkv", "start_unix": start, "steps": steps });
        let _ = fs::write(out.join("film.json"), serde_json::to_string_pretty(&timeline).unwrap_or_default() + "\n");
    }
    let _ = link.send(b"stty echo 2>/dev/null\n");
    link.drain();

    let record = Record {
        format: FORMAT,
        run: out.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(),
        tablet: p.tablet.name.clone(),
        link: format!("USB serial {}; {}", port.display(), notes.join("; ")),
        started,
        tasks: done.into_iter().map(|d| d.run).collect(),
    };
    let json = serde_json::to_string_pretty(&record).map_err(|e| e.to_string())?;
    fs::write(out.join("record.json"), json + "\n").map_err(|e| e.to_string())?;
    println!("{}: record {}", j.title, out.join("record.json").display());
    Ok(())
}

fn swipe_script(p: &Profile, from: [i32; 2], to: [i32; 2], seconds: u64) -> String {
    format!(
        "set -eu\nA=/run/potato/potato-agent\nrm -f /run/potato/events.bin /run/potato/events.bin.start\n\
         $A record --dev {dev} --seconds {seconds} --out /run/potato/events.bin >/run/potato/record.log 2>&1 &\nR=$!\nsleep 1\n\
         $A swipe --dev {dev} --from {},{} --to {},{} --ms 500 --steps 15\nwait $R\ncat /run/potato/record.log\n",
        from[0], from[1], to[0], to[1], dev = p.touch.device
    )
}

/// Without a film, a webcam still is taken at the moment itself.
fn still_now(p: &Profile, out: &Path, cams: &[(String, Cam)], when: &str, detail: &mut Vec<String>) {
    if let Some((name, _)) = cams.iter().rev().find(|(n, _)| n.ends_with(&format!("webcam-{when}.jpg"))) {
        if let Err(e) = capture_webcam(p, &out.join(name)) {
            detail.push(format!("webcam {when}: {e}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> Profile {
        toml::from_str(include_str!("../../../profiles/onn-10-pro.toml")).unwrap()
    }

    #[test]
    fn a_screen_point_goes_through_the_onn_calibration_the_way_onn_tap_does() {
        // onn-tap: x = (1920 - sy) * 1199 / 1920, y = sx * 1919 / 1200. The Church Calendar tile
        // at screen 330,920 was tapped that way on 2026-10-23 and opened the calendar.
        assert_eq!(screen_to_touch(&profile(), 330.0, 920.0).unwrap(), [624, 528]);
        assert!(screen_to_touch(&profile(), 1300.0, 10.0).is_err(), "a point off the screen is refused");
    }

    #[test]
    fn an_element_is_placed_from_the_bottom_right_of_its_window() {
        // Lampada maximised on the onn: window at 12,38 sized 776x946 (logical), page 776x946 CSS
        // pixels, dpr 1.5 on a 1.5 monitor. A centre at CSS 100,200 is screen 18+150, 57+300.
        let aim = serde_json::json!({"visible": 1, "on_page": true, "reaches": true, "cx": 100.0, "cy": 200.0,
                                     "iw": 776.0, "ih": 946.0, "dpr": 1.5, "label": "Church Calendar"});
        let (x, y, _) = element_on_screen(&aim, [12.0, 38.0, 776.0, 946.0], 1.5).unwrap();
        assert_eq!((x.round(), y.round()), (168.0, 357.0));
        // A bar above the page (here 40 logical pixels) pushes the page down, not the window.
        let short = serde_json::json!({"visible": 1, "on_page": true, "reaches": true, "cx": 100.0, "cy": 200.0,
                                       "iw": 776.0, "ih": 906.0, "dpr": 1.5, "label": "x"});
        assert_eq!(element_on_screen(&short, [12.0, 38.0, 776.0, 946.0], 1.5).unwrap().1.round(), 417.0);
    }

    #[test]
    fn an_element_that_is_covered_or_off_the_page_is_not_tapped() {
        let base = |k: &str, v: serde_json::Value| {
            let mut a = serde_json::json!({"visible": 1, "on_page": true, "reaches": true, "cx": 1.0, "cy": 1.0, "iw": 10.0, "ih": 10.0, "dpr": 1.0});
            a[k] = v; a
        };
        assert!(element_on_screen(&base("reaches", false.into()), [0.0; 4], 1.0).unwrap_err().contains("covers"));
        assert!(element_on_screen(&base("on_page", false.into()), [0.0; 4], 1.0).unwrap_err().contains("off the visible page"));
        assert!(element_on_screen(&base("visible", 0.into()), [0.0; 4], 1.0).unwrap_err().contains("no visible element"));
    }

    #[test]
    fn keys_are_labelled_by_order_and_refused_when_the_rows_do_not_match() {
        // Two rows of the onn's keyboard as potato-agent found them on 2026-10-23.
        let found = vec![(1626, 1685, vec![(3, 121), (127, 204), (210, 288), (294, 371)]), (1690, 1749, vec![(3, 81), (87, 163)])];
        let layout = vec![vec!["Tab".to_string(), "q".into(), "w".into(), "e".into()], vec!["Cmp".to_string(), "a".into()]];
        assert_eq!(key_points(&found, &layout, "wa").unwrap(), vec![(249, 1655), (125, 1719)]);
        assert!(key_points(&found, &layout, "z").unwrap_err().contains("no key"));
        let short = vec![found[0].clone()];
        assert!(key_points(&short, &layout, "q").unwrap_err().contains("rows of"));
    }

    #[test]
    fn the_profile_layout_matches_the_keyboard_found_on_the_onn() {
        let kb = profile().keyboard.expect("the onn profile names its keyboard");
        let counts: Vec<usize> = kb.rows.iter().map(|r| r.len()).collect();
        assert_eq!(counts, vec![14, 14, 14, 14, 13, 10], "the rows potato-agent keys found on 2026-10-23");
    }

    #[test]
    fn keyrows_parse_from_the_agent_output() {
        let rows = parse_keyrows("noise\nPOTATO_KEYROW 1497 1555 3-100 107-184\nPOTATO_KEYROW 1561 1620 3-81\n");
        assert_eq!(rows, vec![(1497, 1555, vec![(3, 100), (107, 184)]), (1561, 1620, vec![(3, 81)])]);
    }
}
