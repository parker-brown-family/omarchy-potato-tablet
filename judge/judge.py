#!/usr/bin/env python3
"""Judge a potato run: fill in every observer's answer in record.json, then draw index.html.

    python3 judge/judge.py <run-dir> --profile profiles/onn-10-pro.toml

Exact where the expectation is exact: the sink's text, the app's own state after a journey step,
the number of touch-downs and where they landed. For the framebuffer and the webcam, code finds the region that changed between "before"
(the empty sink) and "after", and Jev reads the crop. The same crop of "before" is asked the same
question as a control, so a word that was already on screen cannot pass.

Jev goes through JevClient (jev-integrations), whose image route carries a colour canary: if the
model was not shown the pictures, every answer abstains and the observer reads "not observed".
Standard library only, plus ffmpeg for pictures.
"""
from __future__ import annotations

import argparse
import gzip
import html
import json
import os
import struct
import subprocess
import sys
import tomllib
from pathlib import Path

JEV_REPO = Path(os.environ.get("JEV_INTEGRATIONS", "~/Work/jev-integrations")).expanduser()
sys.path.insert(0, str(JEV_REPO))
from jev import images as jev_images          # noqa: E402
from jev.client import JevClient              # noqa: E402
from jev.dotenv import load as load_dotenv    # noqa: E402
from jev.ports.hosted import HostedPort       # noqa: E402

# Provisional: no labelled cases have measured these yet (ask-jev check needs ten or more).
AFTER_PASS = 0.8     # the after crop must read as showing the word at least this strongly
BEFORE_MAX = 0.2     # and the before crop, the control, at most this strongly
AFTER_FAIL = 0.2     # an after crop at or below this is a clear "not there"
TAP_TOLERANCE = 2    # digitizer units: synthetic taps should land exactly where they were aimed
EVENT = struct.Struct("<qqHHi")


# ---- exact observers ------------------------------------------------------------------------------

def touch_downs(path: Path, start: tuple[int, int] | None = None) -> list[tuple[int, int]]:
    """First position of every contact, in order. Positions may arrive before the tracking id, and
    a position equal to the slot's last one is not sent at all, so the recording starts from where
    the slot stood (`start`) when that is known, and from nothing when it is not."""
    raw = path.read_bytes()
    downs, open_contact, placed = [], False, False
    x, y = start if start else (None, None)
    for i in range(0, len(raw) - EVENT.size + 1, EVENT.size):
        _, _, typ, code, val = EVENT.unpack_from(raw, i)
        if (typ, code) == (3, 53):
            x = val
        elif (typ, code) == (3, 54):
            y = val
        elif (typ, code) == (3, 57):
            open_contact, placed = val >= 0, False
        elif (typ, code) == (0, 0) and open_contact and not placed and x is not None and y is not None:
            downs.append((x, y))
            placed = True
    return downs


def judge_input_events(run: Path, obs: dict, task: dict, expect_taps: int) -> None:
    if not obs["evidence"]:
        return answer(obs, "not_observed", "no event recording came back from the tablet", "exact")
    events = next(f for f in obs["evidence"] if f.endswith(".bin"))
    start = None
    for f in obs["evidence"]:
        if f.endswith(".start"):
            words = (run / f).read_text().split()
            start = (int(words[0]), int(words[1])) if len(words) == 2 else None
    downs = touch_downs(run / events, start)
    aimed = [tuple(p) for p in task["aim"]["points"]]
    if len(downs) != expect_taps:
        return answer(obs, "fail", f"{len(downs)} touch-downs recorded, {expect_taps} expected: {downs}", "exact")
    off = [(d, a) for d, a in zip(downs, aimed) if abs(d[0] - a[0]) > TAP_TOLERANCE or abs(d[1] - a[1]) > TAP_TOLERANCE]
    if off:
        return answer(obs, "fail", f"touch-downs away from their aim: {off}", "exact")
    answer(obs, "pass", f"{len(downs)} touch-downs, each within {TAP_TOLERANCE} of its aim: {downs}", "exact")


def judge_app_state(run: Path, obs: dict) -> None:
    """A journey step's exact observer: the app's answer to the step's check, asked after the act."""
    if not obs["evidence"]:
        return answer(obs, "not_observed", "the app's state did not come back from the tablet", "exact")
    st = json.loads((run / obs["evidence"][0]).read_text())
    if "error" in st:
        return answer(obs, "not_observed", f"the app could not be asked: {st['error']}", "exact")
    if st.get("value") is True:
        return answer(obs, "pass", f"`{st['expression']}` is true", "exact")
    answer(obs, "fail", f"`{st['expression']}` gave {json.dumps(st.get('value'))}", "exact")


def judge_sink(run: Path, obs: dict, expected: str) -> None:
    if not obs["evidence"]:
        return answer(obs, "not_observed", "the sink's file did not come back from the tablet", "exact")
    got = (run / obs["evidence"][0]).read_bytes().decode("utf-8", "replace")
    if got == expected:
        return answer(obs, "pass", f"received exactly {got!r}", "exact")
    answer(obs, "fail", f"received {got!r}, expected {expected!r}", "exact")


# ---- pictures -------------------------------------------------------------------------------------

def ffmpeg(*args: str, stdin: bytes | None = None) -> bytes:
    r = subprocess.run(["ffmpeg", "-hide_banner", "-loglevel", "error", *args], input=stdin, capture_output=True)
    if r.returncode != 0:
        raise RuntimeError(r.stderr.decode(errors="replace").strip())
    return r.stdout


SHORT_SIDE_MIN = 288   # the API refuses a short side of 256 and takes 280 (measured 2026-10-23,
                       # OpenRouter, jev-1.14: 279 refused, 280 taken, whatever the area; the docs say
                       # 256; parker-brown-family/jev-integrations#28). 288 is the canary's size
LONG_SIDE_MAX = 1568


def fit_for_jev(w: int, h: int) -> str:
    """A scale filter that brings a crop inside the API's image limits. A 132 x 81 crop of a typed
    word was refused for size, and so was the same crop at 417 x 256. Never shrinks a crop that
    already fits; enlarges the short side to SHORT_SIDE_MIN, keeping the long side within the max."""
    k = max(1.0, SHORT_SIDE_MIN / min(w, h))
    k = min(k, LONG_SIDE_MAX / max(w, h))
    return f"scale={max(SHORT_SIDE_MIN, round(w * k))}:{max(SHORT_SIDE_MIN, round(h * k))}:flags=lanczos"


def framebuffer_png(raw: bytes, fb: dict, out: Path, crop: tuple[int, int, int, int] | None = None) -> None:
    w_padded = fb["stride"] // 4
    vf = f"crop={fb['width']}:{fb['height']}:0:0"
    if crop:
        x, y, w, h = crop
        vf = f"crop={w}:{h}:{x}:{y},{fit_for_jev(w, h)}"
    ffmpeg("-f", "rawvideo", "-pixel_format", "rgba", "-video_size", f"{w_padded}x{fb['height']}", "-i", "-",
           "-vf", vf, "-pix_fmt", "rgb24", "-frames:v", "1", "-y", str(out), stdin=raw)


def changed_box(before: bytes, after: bytes, width: int, height: int, stride: int, bpp: int, threshold: int,
                within: tuple[int, int, int, int] | None = None):
    """Bounding box (x, y, w, h) of the pixels that changed by more than threshold, or None.

    `within` limits the search to one rectangle, so a clock ticking over or a pointer moving
    elsewhere on the screen cannot widen the box."""
    rx, ry, rw, rh = within or (0, 0, width, height)
    rx, ry = max(0, rx), max(0, ry)
    rw, rh = min(width - rx, rw), min(height - ry, rh)
    x0, y0, x1, y1 = width, height, -1, -1
    for row in range(ry, ry + rh):
        a = before[row * stride + rx * bpp: row * stride + (rx + rw) * bpp]
        b = after[row * stride + rx * bpp: row * stride + (rx + rw) * bpp]
        if a == b:
            continue
        for c in range(rw):
            i = c * bpp
            if max(abs(a[i + k] - b[i + k]) for k in range(min(bpp, 3))) > threshold:
                col = rx + c
                x0, x1, y0, y1 = min(x0, col), max(x1, col), min(y0, row), max(y1, row)
    if x1 < 0:
        return None
    return x0, y0, x1 - x0 + 1, y1 - y0 + 1


def screen_box(gray: bytes, w: int, h: int, fb_aspect: float):
    """The tablet's screen in a small grayscale webcam frame: the rows and columns that are mostly
    lit. A bright cable or reflection is too thin to pass. None when the box's shape does not match
    the screen's, which means the camera cannot see the screen properly."""
    lit = [[gray[y * w + x] > 40 for x in range(w)] for y in range(h)]
    rows = [y for y in range(h) if sum(lit[y]) > 0.3 * w]
    cols = [x for x in range(w) if sum(lit[y][x] for y in range(h)) > 0.3 * h]
    if not rows or not cols:
        return None
    x0, y0, x1, y1 = min(cols), min(rows), max(cols) + 1, max(rows) + 1
    aspect = (x1 - x0) / (y1 - y0)
    if abs(aspect - fb_aspect) / fb_aspect > 0.1:
        return None
    return x0, y0, x1 - x0, y1 - y0


def pad(box, margin: int, width: int, height: int):
    x, y, w, h = box
    x0, y0 = max(0, x - margin), max(0, y - margin)
    x1, y1 = min(width, x + w + margin), min(height, y + h + margin)
    return x0, y0, x1 - x0, y1 - y0


def gray_small(jpg: Path, w: int, h: int) -> bytes:
    return ffmpeg("-i", str(jpg), "-vf", f"scale={w}:{h},format=gray", "-f", "rawvideo", "-")


# ---- Jev ------------------------------------------------------------------------------------------

def word_questions(word_path: str) -> dict:
    def q(image: str) -> dict:
        return {
            "type": "noul",
            "instructions": {
                "question": f"Does the picture in `{image}` show the word in `{word_path}` written out as typed text, every letter in order?",
                "focus": "Typed characters in a terminal or text field count. Letters printed on separate keys of an on-screen keyboard do not.",
            },
            "criteria": {
                "true": {"what": "The whole word can be read as one run of typed characters, possibly followed by a cursor"},
                "false": {"what": "The word is absent, cut off, or misspelled", "not_for": "Single letters on keyboard keys"},
            },
        }
    return {"after_shows_word": q("after"), "before_shows_word": q("before")}


def client() -> JevClient:
    load_dotenv(JEV_REPO / ".env")
    key = "OPENROUTER_API_KEY" if os.environ.get("OPENROUTER_API_KEY") else "TYPESAFE_API_KEY"
    return JevClient("https://openrouter.ai/api", port=HostedPort(images="state"), api_key_env=key, timeout_s=30.0)


def screen_questions(description_path: str) -> dict:
    """A journey step's picture question: does this screen show what the step should lead to? The
    description names what must be visible and, where the screen before would otherwise pass, what
    does not count; it is read literally."""
    def q(image: str) -> dict:
        return {
            "type": "noul",
            "instructions": {
                "question": f"Does the picture in `{image}` show what `{description_path}` describes?",
                "focus": "The picture is part of a tablet app's screen. Every part of the description must be visible, and anything the description says does not count must not be what makes it true.",
            },
            "criteria": {
                "true": {"what": "Everything the description names can be seen in the picture"},
                "false": {"what": "Something the description names is missing, or the picture shows a different screen"},
            },
        }
    return {"after_matches": q("after"), "before_matches": q("before")}


def ask_word(jev: JevClient, before_png: Path, after_png: Path, word: str) -> tuple[str, str]:
    ref_b, asset_b = jev_images.asset_path(before_png, "before")
    ref_a, asset_a = jev_images.asset_path(after_png, "after")
    state = {"before": ref_b, "after": ref_a, "expected_word": word}
    got = jev.ask_many(state, word_questions("expected_word"), assets={ref_b: asset_b, ref_a: asset_a})
    return decide(got["after_shows_word"], got["before_shows_word"], "showing the word")


def ask_screen(jev: JevClient, before_png: Path, after_png: Path, description: str) -> tuple[str, str]:
    ref_b, asset_b = jev_images.asset_path(before_png, "before")
    ref_a, asset_a = jev_images.asset_path(after_png, "after")
    state = {"before": ref_b, "after": ref_a, "expected_screen": description}
    got = jev.ask_many(state, screen_questions("expected_screen"), assets={ref_b: asset_b, ref_a: asset_a})
    return decide(got["after_matches"], got["before_matches"], "matching the description")


def decide(a, b, what: str) -> tuple[str, str]:
    """After must read high and before, the control, low. Anything else is not a pass."""
    if not (a.usable and b.usable):
        reason = a.reason if not a.usable else b.reason
        return "not_observed", f"Jev abstained ({reason})"
    detail = f"Jev: after {a.value:.2f}, before (control) {b.value:.2f}; provisional thresholds {AFTER_PASS}/{BEFORE_MAX}"
    if a.value >= AFTER_PASS and b.value <= BEFORE_MAX:
        return "pass", detail
    if a.value <= AFTER_FAIL:
        return "fail", detail
    if b.value > BEFORE_MAX:
        return "not_observed", detail + f"; the control already read as {what}"
    return "not_observed", detail + "; between the thresholds, a case for a person"


def judge_framebuffer(run: Path, obs: dict, fb: dict, ask, region, sub: str = ""):
    """Answer the framebuffer observer, and return the changed box (framebuffer pixels) or None.
    `ask(before_png, after_png)` is the Jev question, or None without a key; files go under `sub`."""
    out = (lambda f: f"{sub}/{f}") if sub else (lambda f: f)
    names = [n for n in obs["evidence"] if n.endswith(".rgba.gz")]   # the captures, not an earlier judge's crops
    obs["evidence"] = list(names)
    if len(names) != 2:
        answer(obs, "not_observed", "a framebuffer capture is missing", "exact")
        return None
    before, after = (gzip.decompress((run / n).read_bytes()) for n in names)
    framebuffer_png(before, fb, run / out("framebuffer-before.png"))
    framebuffer_png(after, fb, run / out("framebuffer-after.png"))
    box = changed_box(before, after, fb["width"], fb["height"], fb["stride"], 4, 24, region)
    where = "inside the window" if region else "anywhere on the screen"
    if box is None:
        answer(obs, "fail", f"nothing changed {where}", "exact")
        return None
    crop = pad(box, 24, fb["width"], fb["height"])
    framebuffer_png(before, fb, run / out("framebuffer-before-crop.png"), crop)
    framebuffer_png(after, fb, run / out("framebuffer-after-crop.png"), crop)
    obs["evidence"] += [out(f) for f in ("framebuffer-before.png", "framebuffer-after.png", "framebuffer-before-crop.png", "framebuffer-after-crop.png")]
    if ask is None:
        answer(obs, "not_observed", f"changed region {crop}; no Jev key to read it", "exact")
        return crop
    verdict, detail = ask(run / out("framebuffer-before-crop.png"), run / out("framebuffer-after-crop.png"))
    answer(obs, verdict, f"changed region x={crop[0]} y={crop[1]} {crop[2]}x{crop[3]}. {detail}", "diff + jev")
    return crop


def judge_webcam(run: Path, obs: dict, ask, fb: dict, region, fb_box, sub: str = "") -> None:
    """The framebuffer says where the change is; the webcam says whether the glass shows it. A
    diff of two webcam frames finds sensor noise everywhere (measured 2026-10-23), so the crop
    comes from the framebuffer's changed box, carried onto the screen as the camera sees it."""
    out = (lambda f: f"{sub}/{f}") if sub else (lambda f: f)
    names = [n for n in obs["evidence"] if n.endswith(".jpg")]       # the captures, not an earlier judge's crops
    obs["evidence"] = list(names)
    if len(names) != 2:
        return answer(obs, "not_observed", "a webcam frame is missing", "exact")
    sw, sh = 270, 480   # the upright frame is 1080 x 1920; a quarter-size grid is enough to find a word
    after = gray_small(run / names[1], sw, sh)
    if sum(after) / len(after) < 8:
        return answer(obs, "not_observed", "the frame is too dark to read", "exact")
    screen = screen_box(after, sw, sh, fb["width"] / fb["height"])
    if screen is None:
        return answer(obs, "not_observed", "the screen could not be found in the frame", "exact")
    target = fb_box or region
    if target is None:
        return answer(obs, "not_observed", "the framebuffer did not say where to look", "exact")
    sx, sy, sww, shh = (v * 4 for v in screen)          # the screen in full-size upright pixels
    kx, ky = sww / fb["width"], shh / fb["height"]
    x, y = int(sx + target[0] * kx), int(sy + target[1] * ky)
    x, y, w, h = pad((x, y, int(target[2] * kx) + 1, int(target[3] * ky) + 1), 24, 1080, 1920)
    for n, crop in zip(names, (out("webcam-before-crop.png"), out("webcam-after-crop.png"))):
        ffmpeg("-i", str(run / n), "-vf", f"crop={w}:{h}:{x}:{y},{fit_for_jev(w, h)}", "-frames:v", "1", "-y", str(run / crop))
    obs["evidence"] += [out("webcam-before-crop.png"), out("webcam-after-crop.png")]
    if ask is None:
        return answer(obs, "not_observed", "no Jev key to read the frame", "exact")
    verdict, detail = ask(run / out("webcam-before-crop.png"), run / out("webcam-after-crop.png"))
    answer(obs, verdict, f"changed region x={x} y={y} {w}x{h}. {detail}", "diff + jev")


# ---- the record and the page ----------------------------------------------------------------------

def answer(obs: dict, verdict: str, detail: str, by: str) -> None:
    assert verdict in ("pass", "fail", "not_observed", "not_attempted"), verdict
    obs["answer"], obs["detail"], obs["judged_by"] = verdict, detail, by


LABELS = {"pass": "pass", "fail": "fail", "not_observed": "not observed", "not_attempted": "not attempted", None: "not judged"}


def page(record: dict) -> str:
    def img(run_file: str, label: str) -> str:
        return f'<figure><img src="{html.escape(run_file)}" alt="{html.escape(label)}"><figcaption>{html.escape(label)}</figcaption></figure>'
    parts = []
    for t in record["tasks"]:
        obs_html = "".join(
            f'<div class="ob {o["answer"] or "none"}"><b>{html.escape(name.replace("_", " "))}</b>'
            f'<span class="v">{LABELS[o["answer"]]}</span><p>{html.escape(o.get("detail") or "")}</p>'
            f'<p class="exp">expected: {html.escape(o["expected"])}</p></div>'
            for name, o in t["observers"].items())
        pics = [(f, f.replace("-", " ").rsplit(".", 1)[0]) for o in t["observers"].values() for f in o["evidence"]
                if f.endswith((".png", ".jpg")) and ("crop" in f or Path(f).name.startswith("webcam"))]
        aim = t.get("aim") or {}
        parts.append(
            f'<section class="task"><h2>{html.escape(t["title"])}</h2>'
            f'<p class="how">{t["kind"]} · {t["input"]} input · acted: {t["acted"]} · aim: {html.escape(aim.get("provenance", "none"))}</p>'
            f'<p class="how">calibration: {html.escape(t.get("calibration") or "none")}</p>'
            f'<div class="obs">{obs_html}</div><div class="frames">{"".join(img(f, l) for f, l in pics)}</div>'
            f'<p class="how">{html.escape(t.get("act_detail") or "")}</p></section>')
    return f"""<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>{html.escape(record["run"])}</title><style>
:root{{--bg:#0f1117;--panel:#171a22;--line:#2a2f3b;--fg:#e6e6e6;--dim:#9aa0ab;--mono:ui-monospace,monospace}}
body{{margin:0;background:var(--bg);color:var(--fg);font:15px/1.5 system-ui,sans-serif}} .wrap{{max-width:960px;margin:0 auto;padding:28px 20px 60px}}
h1{{font-size:22px;margin:0 0 4px}} .meta,.how{{color:var(--dim);font:12.5px var(--mono)}}
.task{{background:var(--panel);border:1px solid var(--line);border-radius:12px;padding:18px;margin:18px 0}}
.obs{{display:grid;grid-template-columns:repeat(auto-fit,minmax(200px,1fr));gap:10px;margin:12px 0}}
.ob{{border:1px solid var(--line);border-radius:8px;padding:8px 10px;font-size:13px}} .ob p{{margin:4px 0 0}} .ob .exp{{color:var(--dim)}}
.ob b{{display:block;font:700 11px var(--mono);letter-spacing:.08em;text-transform:uppercase;color:var(--dim)}}
.v{{font:700 12px var(--mono)}} .pass .v{{color:#2fb67c}} .fail .v{{color:#e0684b}} .not_observed .v,.none .v{{color:#8b8a80}} .not_observed,.none{{border-style:dashed}}
.not_attempted .v{{color:#5b6170}} .frames{{display:grid;grid-template-columns:repeat(auto-fit,minmax(200px,1fr));gap:10px}}
figure{{margin:0}} img{{width:100%;border-radius:8px;border:1px solid var(--line);background:#000}} figcaption{{color:var(--dim);font:11px var(--mono)}}
</style></head><body><div class="wrap"><h1>Diagnostic run · {html.escape(record["tablet"])}</h1>
<div class="meta">{html.escape(record["run"])} · {html.escape(record["started"])} · {html.escape(record["link"])}</div>
{"".join(parts)}</div></body></html>
"""


def guarded(obs: dict, fn, *args):
    """Run one observer's judgment. If the judge itself breaks, that observer could not be read:
    it answers not_observed with the error, and the other observers are still judged."""
    try:
        return fn(*args)
    except Exception as e:   # noqa: BLE001 — any failure here is a fact about this observer only
        answer(obs, "not_observed", f"the judge failed: {type(e).__name__}: {e}", "error")
        return None


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("run", type=Path)
    ap.add_argument("--profile", type=Path, required=True)
    ap.add_argument("--no-jev", action="store_true", help="exact checks only")
    a = ap.parse_args()
    record = json.loads((a.run / "record.json").read_text())
    profile = tomllib.loads(a.profile.read_text())
    task_files = {p.stem: tomllib.loads(p.read_text()) for p in (Path(__file__).resolve().parent.parent / "tasks").glob("*.toml")}
    jev = None if a.no_jev else client()
    for t in record["tasks"]:
        obs = t["observers"]
        if t["acted"] is not True:
            for o in obs.values():
                answer(o, "not_attempted", t.get("act_detail") or "the act did not run", "exact")
            continue
        region = tuple(t["region"]) if t.get("region") else None
        if "app_state" in obs:
            # A journey step: its files sit in their own directory, and its pictures are asked
            # whether they show what the step should lead to.
            sub = str(Path(next(f for o in obs.values() for f in o["evidence"])).parent)
            ask_fb = (lambda b, af, d=obs["framebuffer"]["expected"]: ask_screen(jev, b, af, d)) if jev else None
            ask_cam = (lambda b, af, d=obs["webcam"]["expected"]: ask_screen(jev, b, af, d)) if jev else None
            guarded(obs["input_events"], judge_input_events, a.run, obs["input_events"], t, len(t["aim"]["points"]))
            guarded(obs["app_state"], judge_app_state, a.run, obs["app_state"])
            fb_box = guarded(obs["framebuffer"], judge_framebuffer, a.run, obs["framebuffer"], profile["framebuffer"], ask_fb, region, sub)
            guarded(obs["webcam"], judge_webcam, a.run, obs["webcam"], ask_cam, profile["framebuffer"], region, fb_box, sub)
            continue
        spec = task_files[t["task"]]
        word = spec["expect"]["sink_text"]
        ask = (lambda b, af: ask_word(jev, b, af, word)) if jev else None
        guarded(obs["input_events"], judge_input_events, a.run, obs["input_events"], t, spec["expect"]["taps"])
        guarded(obs["sink"], judge_sink, a.run, obs["sink"], word)
        fb_box = guarded(obs["framebuffer"], judge_framebuffer, a.run, obs["framebuffer"], profile["framebuffer"], ask, region)
        guarded(obs["webcam"], judge_webcam, a.run, obs["webcam"], ask, profile["framebuffer"], region, fb_box)
    (a.run / "record.json").write_text(json.dumps(record, indent=2) + "\n")
    (a.run / "index.html").write_text(page(record))
    for t in record["tasks"]:
        print(t["title"] + ": " + ", ".join(f"{k} {LABELS[o['answer']]}" for k, o in t["observers"].items()))
    print("page:", a.run / "index.html")
    return 0


if __name__ == "__main__":
    sys.exit(main())
