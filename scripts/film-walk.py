#!/usr/bin/env python3
"""Cut footage from a filmed, judged journey: the raw material for a video, not the video.

    python3 scripts/film-walk.py <run-dir> --profile profiles/onn-10-pro.toml

Writes, beside the run's record:

  walk-camera.mp4  the webcam film, upright, as H.264: the tablet doing it with nobody touching it
  walk-split.mp4   what the tablet drew (left, from the framebuffer captures) beside what the camera
                   saw (right), with a ring wherever a synthetic finger landed, the step's title, and
                   the four observers' answers once the judge has given them
  walk-chapters.txt  each step's tap moment, for an editor's timeline

The left side is not a screen recording. It is the framebuffer before each step, switched to the
framebuffer after it a moment past the tap, so it shows the right pictures at about the right time;
the camera side is continuous. Needs the run's film.json (potato journey --film) and record.json.
Standard library only, plus ffmpeg.
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
import tomllib
from pathlib import Path

FONT = "/usr/share/fonts/TTF/JetBrainsMonoNerdFont-Bold.ttf"
SWITCH_AFTER_TAP = 1.2   # seconds after the last tap that the left side changes to "after"
RING = 110               # the tap marker, in framebuffer pixels
MARK = {"pass": "✓", "fail": "✗", "not_observed": "?", "not_attempted": "–", None: "·"}


def ffmpeg(*args: str) -> None:
    r = subprocess.run(["ffmpeg", "-hide_banner", "-loglevel", "error", "-y", *args])
    if r.returncode != 0:
        sys.exit(f"ffmpeg failed: {' '.join(args[:6])}…")


def touch_to_screen(p: dict, x: int, y: int) -> tuple[float, float]:
    """The calibration's libinput matrix, forwards: a touchscreen point onto the screen."""
    a, b, c, d, e, f = p["calibration"]["matrix"]
    u, v = x / p["touch"]["x_max"], y / p["touch"]["y_max"]
    return (a * u + b * v + c) * p["framebuffer"]["width"], (d * u + e * v + f) * p["framebuffer"]["height"]


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("run", type=Path)
    ap.add_argument("--profile", type=Path, required=True)
    a = ap.parse_args()
    run, prof = a.run, tomllib.loads(a.profile.read_text())
    timeline = json.loads((run / "film.json").read_text())
    record = json.loads((run / "record.json").read_text())
    tasks = {t["task"].split("/", 1)[-1]: t for t in record["tasks"]}
    steps = timeline["steps"]
    gap = 0.53  # hold plus gap between taps in the journey, seconds

    # 1 · the camera, upright
    cam_filter = prof["webcam"]["filter"]
    if not (run / "walk-camera.mp4").exists():
        ffmpeg("-i", str(run / "film.mkv"), "-vf", f"setpts=PTS-STARTPTS,{cam_filter}", "-c:v", "libx264", "-crf", "20",
               "-preset", "medium", "-pix_fmt", "yuv420p", str(run / "walk-camera.mp4"))
    probe = subprocess.run(["ffprobe", "-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", str(run / "walk-camera.mp4")],
                           capture_output=True, text=True)
    total = float(probe.stdout.strip())

    # 2 · the framebuffer side. The screen after a step is the screen before the next one, so the
    # left side is the first step's "before", then each step's "after" from a moment past its taps.
    concat, marks, cues, chapters, switches = [], [], [], [], []
    rows = []
    for s in steps:
        t = tasks[s["dir"].split("-", 1)[1]]
        pts = (t.get("aim") or {}).get("points") or []
        if not (run / s["dir"] / "framebuffer-after.png").exists():
            continue
        rows.append((s, t, pts, s["act"] + SWITCH_AFTER_TAP + max(0, len(pts) - 1) * gap))
    first_before = run / rows[0][0]["dir"] / "framebuffer-before.png"
    concat.append((first_before, rows[0][3]))
    for i, (s, t, pts, switch) in enumerate(rows):
        until = rows[i + 1][3] if i + 1 < len(rows) else total
        concat.append((run / s["dir"] / "framebuffer-after.png", max(0.1, until - switch)))
        swipe = "swipe" in (t["aim"]["source"] or "")
        for k, (x, y) in enumerate(pts):
            sx, sy = touch_to_screen(prof, x, y)
            marks.append((s["act"] + k * gap, sx, sy, swipe))
        obs = t["observers"]
        verdict = "   ".join(f"{name} {MARK[obs[key]['answer']]}" for key, name in
                             (("input_events", "touch"), ("app_state", "app"), ("framebuffer", "screen"), ("webcam", "camera")) if key in obs)
        start = 0.0 if i == 0 else max(0.0, s["before"] - 0.5)
        cues.append([start, None, f"{i + 1} of {len(rows)}   {t['title']}", s["after"], verdict])
        chapters.append(f"{s['act']:7.1f} s  step {i + 1}: {t['title']}")
    for i, c in enumerate(cues):   # each title holds until the next one begins: no two at once
        c[1] = cues[i + 1][0] if i + 1 < len(cues) else total
    lst = run / ".fb-timeline.txt"
    lst.write_text("".join(f"file '{p.resolve()}'\nduration {d:.3f}\n" for p, d in concat) + f"file '{concat[-1][0].resolve()}'\n")
    (run / "walk-chapters.txt").write_text("\n".join(chapters) + "\n")

    # The tap ring: drawn once, laid over the framebuffer side at each tap for half a second.
    ring = run / ".ring.png"
    ffmpeg("-f", "lavfi", "-i", f"color=c=black@0.0:s={RING}x{RING},format=rgba", "-vf",
           f"geq=r=255:g=214:b=10:a='if(between(hypot(X-{RING / 2},Y-{RING / 2}),{RING / 2 - 12},{RING / 2 - 3}),235,0)'",
           "-frames:v", "1", str(ring))

    fb_w, fb_h = prof["framebuffer"]["width"], prof["framebuffer"]["height"]
    graph = [f"[0:v]fps=30,scale={fb_w}:{fb_h},format=yuv420p[fb0]"]
    last = "fb0"
    for n, (t0, x, y, swipe) in enumerate(marks):
        if swipe:
            # A swipe moves 320 CSS pixels up over half a second, starting where it was aimed.
            xexpr, yexpr = f"{x - RING / 2:.0f}", f"{y - RING / 2:.0f}-(t-{t0:.2f})/0.5*480"
            dur = 0.75
        else:
            xexpr, yexpr, dur = f"{x - RING / 2:.0f}", f"{y - RING / 2:.0f}", 0.5
        graph.append(f"[{last}][1:v]overlay=x='{xexpr}':y='{yexpr}':enable='between(t,{t0:.2f},{t0 + dur:.2f})'[fb{n + 1}]")
        last = f"fb{n + 1}"
    graph.append(f"[2:v]setpts=PTS-STARTPTS,{cam_filter},scale=-2:{fb_h},format=yuv420p[cam]")
    graph.append(f"[{last}][cam]hstack=inputs=2[both]")
    last = "both"
    for n, (t0, t1, title, t_after, verdict) in enumerate(cues):
        tf, vf = run / f".cue-{n}-title.txt", run / f".cue-{n}-verdict.txt"
        tf.write_text(title)
        vf.write_text(verdict)
        graph.append(f"[{last}]drawtext=fontfile={FONT}:textfile='{tf.resolve()}':fontsize=54:fontcolor=white:box=1:boxcolor=black@0.72:boxborderw=22:"
                     f"x=(w-text_w)/2:y=40:enable='between(t,{t0:.2f},{t1:.2f})'[t{n}]")
        graph.append(f"[t{n}]drawtext=fontfile={FONT}:textfile='{vf.resolve()}':fontsize=48:fontcolor=0x7CFC9A:box=1:boxcolor=black@0.72:boxborderw=20:"
                     f"x=(w-text_w)/2:y=150:enable='between(t,{t_after:.2f},{t1:.2f})'[v{n}]")
        last = f"v{n}"
    ffmpeg("-f", "concat", "-safe", "0", "-i", str(lst), "-loop", "1", "-i", str(ring), "-i", str(run / "film.mkv"),
           "-filter_complex", ";".join(graph), "-map", f"[{last}]", "-t", f"{total:.2f}",
           "-c:v", "libx264", "-crf", "21", "-preset", "fast", "-pix_fmt", "yuv420p", str(run / "walk-split.mp4"))
    for f in run.glob(".cue-*.txt"):
        f.unlink()
    print(f"{run / 'walk-camera.mp4'}\n{run / 'walk-split.mp4'}\n{run / 'walk-chapters.txt'}  ({total:.0f} s, {len(marks)} touches)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
