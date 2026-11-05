#!/usr/bin/env python3
"""Run the release binary in --demo mode and capture only its own window, by window ID.

Never captures a display or a region. The game exits by itself via --exit-after-ticks.
"""
import argparse
import os
import re
import subprocess
import sys
import threading
import time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BIN = os.path.join(ROOT, "target", "release", "little-quest")
OUT = os.path.join(ROOT, "target", "captures")
WINDOW_ID = os.path.join(ROOT, "target", "window_id")

# (mode, room) -> seconds to wait after the MARK line before capturing.
WANTED = {
    ("title", "meadow"): 0.4,
    ("playing", "meadow"): 0.5,
    ("wipe", "meadow"): 0.1,
    ("dialogue", "cave"): 1.4,
    ("itemget", "cave"): 0.4,
    ("scrolling", "pass"): 0.35,
    ("playing", "approach"): 0.6,
    ("playing", "d1"): 0.5,
    ("scrolling", "d2"): 0.35,
    ("itemget", "d2"): 0.4,
    ("playing", "d3"): 1.5,
    ("itemget", "d3"): 0.4,
    ("victory", "d3"): 0.6,
}

MARK = re.compile(r"MARK tick=(\d+) mode=(\w+) room=(\w+)")


def build_window_id_tool():
    src = os.path.join(ROOT, "scripts", "window_id.swift")
    if not os.path.exists(WINDOW_ID) or os.path.getmtime(WINDOW_ID) < os.path.getmtime(src):
        subprocess.run(["swiftc", "-O", "-o", WINDOW_ID, src], check=True)


def find_window(pid, timeout=15.0):
    end = time.time() + timeout
    while time.time() < end:
        r = subprocess.run([WINDOW_ID, str(pid)], capture_output=True, text=True)
        if r.returncode == 0 and r.stdout.strip():
            return r.stdout.strip()
        time.sleep(0.1)
    return None


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ticks", type=int, default=3000)
    ap.add_argument("--seed", type=int, default=1)
    args = ap.parse_args()

    os.makedirs(OUT, exist_ok=True)
    build_window_id_tool()
    proc = subprocess.Popen(
        [BIN, "--demo", "--exit-after-ticks", str(args.ticks), "--seed", str(args.seed)],
        stderr=subprocess.PIPE,
        text=True,
    )
    # GPUI stops its display link while a window is fully covered, so a hidden demo gets no ticks
    # and --exit-after-ticks alone would never fire.
    watchdog = threading.Timer(args.ticks / 60 * 3 + 30, proc.terminate)
    watchdog.daemon = True
    watchdog.start()
    try:
        wid = find_window(proc.pid)
        if wid is None:
            print("no window found for the demo process", file=sys.stderr)
            return 1
        print(f"window id {wid}")
        frames = []
        shots = []
        done = set()
        n = 0

        def grab(label):
            nonlocal n
            n += 1
            path = os.path.join(OUT, f"{n:02d}_{label}.png")
            r = subprocess.run(["screencapture", "-x", "-o", "-l", wid, path], capture_output=True, text=True)
            if r.returncode != 0 or not os.path.exists(path):
                print(f"screencapture failed: {r.stderr.strip()}", file=sys.stderr)
                os._exit(3)
            shots.append(path)

        timers = []
        for line in proc.stderr:
            line = line.rstrip()
            if line.startswith("FRAMES"):
                frames.append(line)
                continue
            print(line)
            m = MARK.search(line)
            if m:
                key = (m.group(2), m.group(3))
                if key in WANTED and key not in done:
                    done.add(key)
                    t = threading.Timer(WANTED[key], grab, args=(f"{key[0]}_{key[1]}",))
                    t.start()
                    timers.append(t)
        for t in timers:
            t.join()
        code = proc.wait()
        print(f"exit code {code}")
        print(f"{len(shots)} captures in {OUT}")
        if frames:
            print("last FRAMES:", frames[-1])
        return 0 if code == 0 else 1
    finally:
        watchdog.cancel()
        if proc.poll() is None:
            proc.terminate()
            try:
                proc.wait(timeout=3)
            except subprocess.TimeoutExpired:
                proc.kill()


if __name__ == "__main__":
    sys.exit(main())
