"""Tests for the judge's exact parts. Run from the repository root:

    python3 -m unittest judge/test_judge.py
"""
import importlib.util
import struct
import tempfile
import unittest
from pathlib import Path

# Load judge.py by path: the directory is also called judge, and would win an import by name.
_spec = importlib.util.spec_from_file_location("potato_judge", Path(__file__).resolve().parent / "judge.py")
judge = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(judge)

EV = struct.Struct("<qqHHi")


def ev(t, c, v):
    return EV.pack(0, 0, t, c, v)


def tap_like_the_himax(x, y, tid):
    """The order the onn's panel sends a touch-down, recorded 2026-10-22: position before id."""
    down = ev(3, 48, 15) + ev(3, 50, 15) + ev(3, 58, 15) + ev(3, 53, x) + ev(3, 54, y) + ev(3, 57, tid) + ev(1, 330, 1) + ev(0, 0, 0)
    hold = ev(3, 48, 28) + ev(3, 58, 28) + ev(0, 0, 0)
    up = ev(3, 57, -1) + ev(1, 330, 0) + ev(0, 0, 0)
    return down + hold + up


class TouchDowns(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.addCleanup(self.dir.cleanup)

    def test_positions_sent_before_the_tracking_id_still_belong_to_that_touch(self):
        raw = tap_like_the_himax(104, 998, 0) + tap_like_the_himax(144, 563, 1)
        path = Path(self.dir.name) / "events.bin"
        path.write_bytes(raw)
        self.assertEqual(judge.touch_downs(path), [(104, 998), (144, 563)])

    def test_pressure_changes_during_a_hold_are_not_new_touches(self):
        raw = tap_like_the_himax(97, 1417, 5)
        path = Path(self.dir.name) / "events.bin"
        path.write_bytes(raw)
        self.assertEqual(len(judge.touch_downs(path)), 1)


class ChangedBox(unittest.TestCase):
    def frame(self, w, h, marks):
        px = bytearray(w * h)
        for x, y in marks:
            px[y * w + x] = 255
        return bytes(px)

    def test_a_change_outside_the_region_is_ignored(self):
        before = self.frame(20, 20, [])
        after = self.frame(20, 20, [(2, 2), (15, 15)])   # a clock at (2,2), the typed text at (15,15)
        self.assertEqual(judge.changed_box(before, after, 20, 20, 20, 1, 10, within=(10, 10, 10, 10)), (15, 15, 1, 1))

    def test_no_change_is_none_not_an_empty_box(self):
        f = self.frame(8, 8, [(1, 1)])
        self.assertIsNone(judge.changed_box(f, f, 8, 8, 8, 1, 10))


class ScreenBox(unittest.TestCase):
    def test_a_lit_portrait_screen_is_found_and_a_thin_cable_is_not(self):
        w, h = 60, 100
        g = bytearray(w * h)
        for y in range(20, 92):                 # a screen 45 wide and 72 tall: aspect 0.625
            for x in range(8, 53):
                g[y * w + x] = 200
        for y in range(0, 20):                  # a bright cable above it
            g[y * w + 40] = 255
        self.assertEqual(judge.screen_box(bytes(g), w, h, 1200 / 1920), (8, 20, 45, 72))

    def test_a_box_the_wrong_shape_is_not_a_screen(self):
        w, h = 60, 100
        g = bytes([200]) * (w * h)              # everything lit: aspect 0.6 is close, so darken a band
        g = bytearray(g)
        for y in range(0, 60):
            for x in range(w):
                g[y * w + x] = 0
        self.assertIsNone(judge.screen_box(bytes(g), w, h, 1200 / 1920))


if __name__ == "__main__":
    unittest.main()
