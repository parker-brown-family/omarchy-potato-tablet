"""Render docs/ecosystem.md from data/ecosystem.json.

data/ecosystem.json is the result file of a research-delight run on 2026-10-23: every value names
the page it was read from, and a value nobody could confirm is flagged instead of filled in.
Edit the data, not the page, then run:

    python3 scripts/build-ecosystem.py
"""
import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parent.parent
DATA = json.loads((ROOT / "data" / "ecosystem.json").read_text())

GROUPS = [
    ("Tablet layers: install on top of Omarchy 4", [
        ("ragtop", "Ragtop"), ("varlet99-tablet", "Omarchy Tablet (launcher)"),
        ("surface-touch", "omarchy-surface-touch"), ("omarchy-osk", "omarchy-osk"),
        ("yeutterg-tablet", "Omarchy dotfiles for tablets"), ("uripg-tablet", "Omarchy Tablet (convertibles)"),
        ("andreiyurik-tablet-mode", "Tablet Mode for Omarchy"), ("touch-mode", "Omarchy Touch Mode"),
        ("gimbal", "Gimbal"),
    ]),
    ("Hardware runbooks: x86 tablets", [
        ("surface-go-runbook", "Surface Go runbook"), ("surface-book-2", "Surface Book 2 on Omarchy"),
        ("surface-dkms", "Surface touch on the stock kernel"), ("linux-surface", "linux-surface"),
    ]),
    ("Native ARM tablets: Android or iOS replaced", [
        ("fire-max-11", "Amazon Fire Max 11"), ("ipad-6", "iPad 6"),
        ("photo-frame", "$50 photo frame"), ("omarchy-mobile", "Omarchy Mobile (Pixel phones)"),
    ]),
    ("Android kept underneath", [
        ("omarchy-android", "omarchy-android"), ("omalauncher", "Omalauncher"),
    ]),
]
FIELDS = [("gives", "What it gives a tablet"), ("install", "How it goes on"), ("tested", "Tested on"),
          ("slate", "With no keyboard at all"), ("maker", "Who made it"), ("licence", "Licence"),
          ("activity", "When")]

by_id = {u["id"]: u["fields"] for u in DATA["updates"]}


def cell(field):
    if field is None:
        return "_not gathered_"
    text = field["value"].replace("|", "\\|").replace("\n", " ")
    if field.get("flagged"):
        return f"{text} _(unconfirmed)_"
    src = field.get("provenance", {}).get("source")
    return f"{text} ([source]({src}))" if src else text


out = ["# Other people's work", "",
       "Everything here was read at its source on 2026-10-23. A value marked _unconfirmed_ is one",
       "nobody could check, and it is written as a flag rather than a guess.", "",
       "Generated from `data/ecosystem.json` by `scripts/build-ecosystem.py`. Edit the data, not this page.", ""]
for title, subjects in GROUPS:
    out += [f"## {title}", ""]
    for sid, label in subjects:
        f = by_id[sid]
        out += [f"### {label}", ""]
        out += [f"- **{name}:** {cell(f.get(key))}" for key, name in FIELDS]
        out.append("")
out += ["## Things noticed along the way", ""]
out += [f"- {b['note']} ([source]({b['source']}))" for b in DATA.get("bubbles", [])]
out.append("")
(ROOT / "docs" / "ecosystem.md").write_text("\n".join(out))
print(f"{sum(len(s) for _, s in GROUPS)} projects -> docs/ecosystem.md")
