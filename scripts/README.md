# scripts/

Python tooling for Precursor / Xous app development. Standard library only, except `usb_log_monitor.py`, which needs `pyserial`. Python 3.9 or newer.

| Script | What it does |
|---|---|
| `renode_lib.py` | Library. `RenodeSession` owns the Renode process. `RenodeController` drives the monitor: keys, text injection, screenshots, change detection, PDDB setup, app launch. |
| `renode_capture.py` | CLI. Boots Renode, unlocks or formats PDDB, launches an app, runs a capture sequence, and exits non-zero on any failure. |
| `usb_log_monitor.py` | CLI. Streams the USB serial console from a real device (run `usb console` in shellchat first). |

## Environment

| Variable | Meaning | Default |
|---|---|---|
| `XOUS_ROOT` | xous-core checkout. Required unless you pass `--xous-root`. Must exist. | none |
| `RENODE_PATH` | Renode binary | `/Applications/Renode.app/Contents/MacOS/renode` |
| `RENODE_PORT` | Monitor port | `4567` |

## renode_capture.py

```bash
# Blank flash: format PDDB with PIN 'a', then capture calc
python3 scripts/renode_capture.py --xous-root ~/xous-core --init --app calc

# PDDB already formatted
XOUS_ROOT=~/xous-core python3 scripts/renode_capture.py --app calc --screenshots out/calc

# Capture sequence kept in the app's own repo
python3 scripts/renode_capture.py --app myapp --script ../precursor-myapp/capture.py
```

- `--app` is the manifest key, for example `calc` or `cipher-workshop`. The submenu index is read from the build's `services/status/src/app_autogen.rs` (Shellchat is 0, then apps in build order). `--app-index N` overrides it and must be between 0 and 128.
- Screenshots go to `./screenshots/<app>` unless you pass `--screenshots`.
- Renode stdout and stderr go to `$TMPDIR/renode-<port>.log`, or to the path given with `--renode-log`.
- The script refuses to start if the monitor port is already open. Kill the stale Renode first (`lsof -i :4567`).
- Built-in sequences cover c64, calc, carse, flashcards, othello, timers and writer. Any other app gets a single `app.png`.

Exit codes:

| Code | Meaning |
|---|---|
| 0 | Every screenshot captured |
| 1 | One or more captures failed (screenshot failures or failures returned by the plugin) |
| 2 | Configuration error (bad or missing `XOUS_ROOT`, app not in the build, missing plugin) |
| 3 | Renode or monitor error (port in use, Renode exited, cannot connect, PDDB format timeout) |
| 130 | Interrupted |

### Plugin contract (`--script FILE`)

```python
# capture.py in the app repo
import os

def run(ctl, outdir) -> int:
    """ctl: connected renode_lib.RenodeController, app already launched.
    outdir: existing directory. Return the number of failed steps (0 = ok)."""
    ctl.try_screenshot(os.path.join(outdir, "01_initial.png"))
    ctl.timed_key("Down", after=1.0)
    ctl.inject_line("")              # CR / Enter
    ctl.try_screenshot(os.path.join(outdir, "02_next.png"))
    return ctl.screenshot_failures
```

The harness counts screenshot failures on its own through `ctl.screenshot_failures`, so returning that value does not double-count. If `run()` raises, the run counts as failed and Renode is still shut down. `ctl.screenshot()` raises `ScreenshotError`, and `ctl.try_screenshot()` returns `False` instead. Both increment the counter.

## renode_lib.py essentials

```python
from renode_lib import RenodeSession, app_menu_index

with RenodeSession(xous_root) as s:                 # always terminates Renode
    ctl = s.ctl
    ctl.unlock_pddb("a")
    ctl.launch_app(app_menu_index(xous_root, "calc"))
    ctl.inject_string('2+3')                        # \ and " are escaped
    ctl.timed_key("Enter")                          # validated against SCANCODES
    ctl.timed_chord(["ShiftL", "Number6"])
    ctl.wait_for_screen_change(timeout=10)          # ignores the status bar
    ctl.screenshot("out.png")
```

- **Scancodes** (`SCANCODES`) are exactly what `emulation/peripherals/keyboard.cs` maps: `A`-`Z`, `Number0`-`Number9`, `Enter`, `KeypadEnter`, `BackSpace`, `Space`, `ShiftL`, `ShiftR`, `F1`-`F4`, `Up`, `Down`, `Left`, `Right`, `Home` (the menu key), `Tilde`, `OemComma`, `OemPeriod`, `OemQuestion`, `OemPipe`. There is no `Return`, `Escape`, `Tab`, Ctrl or Alt. Anything outside this set raises `ValueError`, where Renode would silently drop it.
- **`timed_key(key, hold_ms=1)`** pauses emulation, presses, runs `hold_ms` of emulated time, releases and resumes. `hold_ms` must be between 1 and 499 (500 ms and above is a long press). Release and `start` are always sent.
- **Injection** (`inject_line`, `inject_string`, `inject_key`) skips key timing entirely. Only characters up to U+00FF work, because the keyboard service keeps only the low byte.
- **Screenshots** drain stale output, take the last inline image, stop on its BEL terminator or after a 0.4 s idle gap, and check for the PNG signature and the IEND chunk.
- **Change detection** hashes only rows 33 and below. The status bar is rows 0-32 (2 x the 16 px Cjk glyph height, `services/gam/src/main.rs:74-89`). Change it with `RenodeController(status_bar_height=...)`.

## usb_log_monitor.py

```bash
pip install pyserial                     # or: pip install '.[serial]'
python3 scripts/usb_log_monitor.py               # auto-detect (/dev/cu.usbmodem* on macOS)
python3 scripts/usb_log_monitor.py --filter calc --save all.log   # --save writes every line, unfiltered
python3 scripts/usb_log_monitor.py --list        # works without pyserial
```

## Tests

```bash
python3 -m venv .venv && .venv/bin/pip install pytest ruff
.venv/bin/python -m pytest -q        # fake TCP monitor, no Renode needed
.venv/bin/ruff check scripts tests
```

## Known limitations

- None of this has been run against real Renode since the rewrite. The tests use a fake monitor.
- `inject_arrow()` and `inject_menu_key()` send ANSI `ESC [ A/B/C/D` and `ESC [ 1 ~` through `InjectKey`. The keyboard service decodes these (`services/keyboard/src/main.rs:906-981`, `esc_match` at 1142), but nobody has tried it in Renode yet. Use `timed_key` if it does not work.
- `init_pddb` waits at least 180 s, then until the screen holds still for 60 s (900 s cap). The 60 s "stable" threshold is a guess. Raise it if the format progress dialog stalls for longer than that.
- Boot still uses a fixed wait (`--boot-wait`, 45 s by default, 60 s with `--init`).
- Writer's preview, export, journal and typewriter shots are skipped with a warning. They depend on an ESC prefix, and the device has no ESC key. They belong in a writer `capture.py` that uses F-keys.
- The derived app index is only correct if the image running in Renode was built from the same `XOUS_ROOT` tree.
- `path add @"<root>"` quoting in the Renode `-e` command follows Renode's quoted-path syntax but has not been tested with paths that contain spaces.
