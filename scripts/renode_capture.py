#!/usr/bin/env python3
"""
Renode screenshot capture for Precursor / Xous apps.

Boots Renode, unlocks (or formats) PDDB, launches an app, runs a capture
sequence, and exits non-zero if any screenshot failed.

    # blank flash: format PDDB, then capture calc
    python3 renode_capture.py --xous-root ~/xous-core --init --app calc

    # PDDB already formatted with PIN 'a'
    XOUS_ROOT=~/xous-core python3 renode_capture.py --app calc --screenshots out/

    # per-app capture sequence that lives in the app's own repo
    python3 renode_capture.py --app myapp --script ../precursor-myapp/capture.py

Plugin contract (--script FILE):
    FILE defines   run(ctl, outdir) -> int
    ctl     a connected renode_lib.RenodeController (app already launched)
    outdir  existing directory for PNGs
    return  number of failed steps the plugin detected (0 = success).
    Screenshot failures are also counted by the harness via
    ctl.screenshot_failures, so a plugin may simply `return ctl.screenshot_failures`.
    Raising an exception counts as a failure; Renode is still shut down.

Exit codes: 0 ok, 1 capture failures, 2 usage/config error, 3 Renode/monitor error.

Environment: XOUS_ROOT, RENODE_PATH, RENODE_PORT (see renode_lib.py).
"""

from __future__ import annotations

import argparse
import importlib.util
import logging
import os
import sys
import tempfile
from typing import Callable, Dict

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from renode_lib import (  # noqa: E402
    DEFAULT_PIN,
    DEFAULT_PORT,
    ConfigError,
    RenodeController,
    RenodeError,
    RenodeSession,
    ScreenshotError,
    app_menu_index,
    reset_flash,
    resolve_xous_root,
)

log = logging.getLogger("renode_capture")

EXIT_OK, EXIT_FAILURES, EXIT_CONFIG, EXIT_RENODE = 0, 1, 2, 3


class Shots:
    """Small helper for capture sequences: ss(name), enter(), wait()."""

    def __init__(self, ctl: RenodeController, outdir: str):
        self.ctl = ctl
        self.outdir = outdir

    def ss(self, name: str) -> bool:
        return self.ctl.try_screenshot(os.path.join(self.outdir, name))

    def enter(self, after: float = 3.0) -> None:
        """CR via inject_line: reliable Enter (timed_key('Enter') also exists)."""
        self.ctl.inject_line("", after=after)

    def wait(self, seconds: float) -> None:
        self.ctl.sleep(seconds)


# ---------------------------------------------------------------------------
# Built-in capture sequences. Each follows the plugin contract. New apps
# should ship a capture.py plugin in their own repo instead of adding here.
# ---------------------------------------------------------------------------

def capture_timers(ctl: RenodeController, outdir: str) -> int:
    s = Shots(ctl, outdir)
    log.info("=== Timers ===")
    s.wait(2)
    s.ss("mode_select.png")
    s.enter()  # Pomodoro
    s.enter()  # start
    s.wait(3)
    s.ss("pomodoro.png")
    ctl.timed_key("S", after=3.0)
    s.ss("settings.png")
    ctl.timed_key("Q", after=3.0)
    ctl.timed_key("Q", after=3.0)
    ctl.timed_key("Down", after=2.0)
    s.enter()  # Stopwatch
    s.enter()  # start
    s.wait(3)
    ctl.timed_key("L", after=2.0)
    s.wait(2)
    ctl.timed_key("L", after=2.0)
    s.wait(1)
    s.ss("stopwatch.png")
    s.enter()  # pause
    ctl.timed_key("Q", after=3.0)
    ctl.timed_key("Down", after=1.0)
    ctl.timed_key("Down", after=1.0)
    s.enter()  # Countdown
    s.ss("countdown_list.png")
    ctl.timed_key("Q", after=3.0)
    return 0


def capture_flashcards(ctl: RenodeController, outdir: str) -> int:
    s = Shots(ctl, outdir)
    log.info("=== Flashcards ===")
    s.ss("deck_list.png")
    ctl.timed_key("M", after=3.0)
    s.ss("deck_menu.png")
    ctl.timed_key("Q", after=2.0)
    s.enter(after=3.0)  # open deck
    s.ss("question.png")
    ctl.timed_key("Space", after=2.0)
    s.ss("answer.png")
    ctl.timed_key("Q", after=2.0)
    ctl.timed_key("I", after=3.0)
    s.ss("import_wait.png")
    return 0


def capture_c64(ctl: RenodeController, outdir: str) -> int:
    s = Shots(ctl, outdir)
    log.info("=== C64 ===")
    s.wait(2)
    s.ss("menu.png")
    ctl.timed_key("Down", after=1.0)
    ctl.timed_key("Down", after=1.0)
    s.ss("menu_boot.png")
    s.enter()
    s.wait(5)
    s.ss("running.png")
    ctl.timed_key("Home", after=3.0)  # menu key
    s.ss("menu_return.png")
    ctl.timed_key("Up", after=1.0)
    s.enter()
    s.wait(3)
    s.ss("import_wait.png")
    return 0


# Writer captures that relied on an ESC-prefixed command. The Precursor has no
# ESC key and Renode's keyboard peripheral has no Escape scancode
# (keyboard.cs MatrixMapping), so these screens cannot be reached as written.
# Writer sources seen in xous-core/apps/writer bind F2 = preview toggle,
# F1 = menu (item 2 = export), F4 = back; move this sequence into the writer
# repo's capture.py once that is confirmed on a real run.
WRITER_SKIPPED = ("editor_preview.png", "export_menu.png", "journal.png", "typewriter.png")


def capture_writer(ctl: RenodeController, outdir: str) -> int:
    s = Shots(ctl, outdir)
    log.info("=== Writer ===")
    s.wait(2)
    s.ss("mode_select.png")
    s.enter()  # -> DocList
    s.wait(2)
    ctl.timed_key("N", after=3.0)  # new document
    s.wait(2)
    for line in (
        "# My Document", "", "This is a **writer** app for Precursor.", "",
        "## Features", "", "- Markdown styling", "- Journal mode", "- Typewriter mode", "",
        "> Built for focused writing", "", "---",
    ):
        ctl.inject_line(line, after=0.5)
    s.wait(2)
    s.ss("editor_edit.png")
    log.warning(
        "Writer: SKIPPED %s - they need an ESC-prefix command the device cannot produce "
        "(no ESC key). Port them to F-keys in a writer capture.py plugin.",
        ", ".join(WRITER_SKIPPED),
    )
    return 0


def capture_calc(ctl: RenodeController, outdir: str) -> int:
    s = Shots(ctl, outdir)
    log.info("=== Calculator ===")
    s.wait(3)
    s.ss("01_initial.png")
    ctl.inject_string("2+3*4")
    s.wait(1)
    s.ss("02_expression.png")
    s.enter(after=2.0)
    s.ss("03_result.png")
    ctl.inject_string("sin(90)")
    s.wait(1)
    s.ss("04_sin90_input.png")
    s.enter(after=2.0)
    s.ss("05_sin90_result.png")
    ctl.inject_string("sqrt(2)")
    s.wait(1)
    s.enter(after=2.0)
    s.ss("06_sqrt2.png")
    ctl.inject_string("(2+3)*4")
    s.wait(1)
    s.ss("07_parens_input.png")
    s.enter(after=2.0)
    s.ss("08_parens_result.png")
    ctl.inject_key("M", after=2.0)  # RPN mode
    s.ss("09_rpn_mode.png")
    ctl.inject_key("2", after=0.5)
    s.ss("10_rpn_digit2.png")
    s.enter(after=1.5)
    s.ss("11_rpn_enter.png")
    ctl.inject_key("3", after=0.5)
    s.ss("12_rpn_digit3.png")
    ctl.inject_key("+", after=1.0)
    s.ss("13_rpn_add.png")
    ctl.inject_key("C", after=0.5)
    ctl.inject_key("2")
    s.enter(after=0.5)
    ctl.inject_key("3")
    ctl.inject_key("*", after=0.5)
    ctl.inject_key("4")
    ctl.inject_key("+", after=1.0)
    s.ss("14_rpn_complex.png")
    ctl.inject_key("C", after=0.5)
    ctl.inject_key("2")
    ctl.timed_chord(["ShiftL", "Number6"], after=1.0)  # sqrt
    s.ss("15_rpn_sqrt.png")
    ctl.inject_key("A", after=1.0)  # angle mode
    s.ss("16_rad_mode.png")
    ctl.inject_key("M", after=1.0)  # back to ALG
    s.ss("17_alg_rad.png")
    ctl.inject_string("42")
    s.enter(after=1.0)
    ctl.inject_key("s", after=0.5)
    ctl.inject_key("0", after=1.0)
    s.ss("18_memory_store.png")
    ctl.inject_string("255")
    s.enter(after=1.0)
    ctl.inject_key("b", after=1.0)
    s.ss("19_hex_mode.png")
    return 0


def capture_carse(ctl: RenodeController, outdir: str) -> int:
    s = Shots(ctl, outdir)
    log.info("=== Carse Automata ===")
    s.wait(3)
    s.ss("01_initial.png")
    for _ in range(5):
        ctl.timed_key("Right", after=0.3)
    for _ in range(3):
        ctl.timed_key("Down", after=0.3)
    s.wait(1)
    s.ss("02_cursor_moved.png")
    for key in ("Space", "Right", "Space", "Right", "Space", "Down", "Space"):
        ctl.timed_key(key, after=1.0 if key == "Space" else 0.3)
    s.wait(1)
    s.ss("03_touched_cells.png")
    for presses, name, settle in (
        (15, "04_simulation_running.png", 2),
        (20, "05_simulation_evolved.png", 2),
        (30, "06_simulation_later.png", 3),
    ):
        for _ in range(presses):
            ctl.timed_key("Space", after=0.3)
        s.wait(settle)
        s.ss(name)
    s.wait(3)
    s.ss("07_paused.png")
    for name in ("08_toward_death.png", "09_extended.png"):
        for _ in range(50):
            ctl.timed_key("Space", after=0.3)
        s.wait(3)
        s.ss(name)
    s.wait(2)
    s.ss("10_final.png")
    return 0


def capture_othello(ctl: RenodeController, outdir: str) -> int:
    s = Shots(ctl, outdir)

    def play_game(ai_wait: float, max_moves: int = 65) -> None:
        for move in range(1, max_moves + 1):
            ctl.timed_key("Right", after=0.3)
            s.enter(after=ai_wait)
            if move % 8 == 0:
                ctl.timed_key("Down", after=0.3)
            if move % 20 == 0:
                log.info("      ...move %d", move)

    def back_to_menu() -> None:
        ctl.timed_key("F4", after=3.0)
        s.enter(after=2.0)
        s.wait(2)

    log.info("=== Othello ===")
    s.wait(3)
    s.ss("01_main_menu.png")
    ctl.timed_key("N", after=3.0)
    s.ss("02_new_game_menu.png")
    levels = (("Number1", "03_easy_start.png", "04_easy_end.png", 2),
              ("Number2", "05_medium_start.png", "06_medium_end.png", 3),
              ("Number3", "07_hard_start.png", "08_hard_end.png", 4),
              ("Number4", "09_expert_start.png", "10_expert_end.png", 5))
    for i, (key, start_png, end_png, ai_wait) in enumerate(levels):
        if i:
            ctl.timed_key("N", after=3.0)
        ctl.timed_key(key, after=5.0)
        s.wait(3)
        s.ss(start_png)
        play_game(ai_wait)
        s.wait(3)
        s.ss(end_png)
        back_to_menu()

    ctl.timed_key("N", after=3.0)
    ctl.timed_key("Number5", after=5.0)
    s.wait(3)
    s.ss("11_two_player.png")
    for _ in range(6):
        ctl.timed_key("Right", after=0.5)
        s.enter(after=1.5)
    s.ss("12_two_player_midgame.png")
    ctl.timed_key("F4", after=3.0)
    s.wait(2)

    ctl.timed_key("N", after=3.0)
    ctl.timed_key("Number1", after=5.0)
    s.wait(3)
    for _ in range(4):
        ctl.timed_key("Right", after=0.5)
        s.enter(after=2.0)
    ctl.timed_key("F1", after=3.0)
    s.ss("13_f1_menu.png")
    ctl.timed_key("F4", after=2.0)

    ctl.timed_key("F1", after=2.0)  # Help(0), MoveHistory(1), Hint(2)
    ctl.timed_key("Down", after=0.5)
    ctl.timed_key("Down", after=0.5)
    s.enter(after=3.0)
    s.ss("14_hint.png")

    ctl.timed_key("F4", after=2.0)
    s.wait(2)
    ctl.timed_key("S", after=3.0)
    s.ss("15_settings.png")
    s.enter(after=2.0)
    s.ss("16_settings_toggled.png")
    ctl.timed_key("F4", after=2.0)

    ctl.timed_key("F1", after=2.0)  # Help(0), NewGame(1), Statistics(2)...
    for _ in range(3):
        ctl.timed_key("Down", after=0.5)
    s.enter(after=3.0)
    s.ss("17_statistics.png")
    ctl.timed_key("F4", after=2.0)

    ctl.timed_key("F1", after=2.0)
    s.enter(after=3.0)
    s.ss("18_help.png")
    ctl.timed_key("F4", after=2.0)
    s.ss("19_final_menu.png")
    return 0


def capture_default(ctl: RenodeController, outdir: str) -> int:
    Shots(ctl, outdir).ss("app.png")
    return 0


BUILTIN: Dict[str, Callable[[RenodeController, str], int]] = {
    "c64": capture_c64,
    "calc": capture_calc,
    "carse": capture_carse,
    "flashcards": capture_flashcards,
    "othello": capture_othello,
    "timers": capture_timers,
    "writer": capture_writer,
}


def load_plugin(path: str) -> Callable[[RenodeController, str], int]:
    """Load run(ctl, outdir) -> int from a Python file."""
    if not os.path.isfile(path):
        raise ConfigError(f"capture script not found: {path}")
    spec = importlib.util.spec_from_file_location("capture_plugin", path)
    if spec is None or spec.loader is None:
        raise ConfigError(f"cannot load capture script: {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    run = getattr(module, "run", None)
    if not callable(run):
        raise ConfigError(f"{path} must define run(ctl, outdir) -> int")
    return run


def run_capture(fn: Callable[[RenodeController, str], int], ctl: RenodeController, outdir: str) -> int:
    """Run a capture function; returns total failures (never raises ScreenshotError)."""
    try:
        reported = fn(ctl, outdir)
    except ScreenshotError:
        reported = 1  # plugin used ctl.screenshot() and let it raise
    if reported is None:
        reported = 0
    if not isinstance(reported, int) or isinstance(reported, bool):
        log.warning("capture function returned %r, expected int; treating as 1 failure", reported)
        reported = 1
    return max(reported, ctl.screenshot_failures)


def parse_args(argv=None) -> argparse.Namespace:
    p = argparse.ArgumentParser(
        description="Renode screenshot automation for Precursor apps",
        epilog="Exit codes: 0 ok, 1 capture failures, 2 config error, 3 Renode error.",
    )
    p.add_argument("--xous-root", default=os.environ.get("XOUS_ROOT"),
                   help="xous-core checkout (default: $XOUS_ROOT)")
    p.add_argument("--init", action="store_true", help="blank the flash and format PDDB first")
    p.add_argument("--pin", default=DEFAULT_PIN, help="PDDB PIN (default: a)")
    p.add_argument("--app", help="app manifest key to launch, e.g. calc")
    p.add_argument("--app-index", type=int,
                   help="'Switch to App' submenu index (default: derived from the build's app_autogen.rs)")
    p.add_argument("--screenshots", help="output directory (default: ./screenshots/<app>)")
    p.add_argument("--script", help="capture plugin: Python file defining run(ctl, outdir) -> int")
    p.add_argument("--port", type=int, default=DEFAULT_PORT, help="monitor port (default: $RENODE_PORT or 4567)")
    p.add_argument("--boot-wait", type=float, help="seconds to wait for boot (default: 60 with --init, else 45)")
    p.add_argument("--renode-log", help="Renode stdout/stderr log (default: $TMPDIR/renode-<port>.log)")
    p.add_argument("-q", "--quiet", action="store_true", help="only warnings and errors")
    args = p.parse_args(argv)
    if args.app_index is not None and not 0 <= args.app_index <= 128:
        p.error("--app-index must be 0..128")
    if args.script and not args.app:
        p.error("--script requires --app")
    return args


def main(argv=None) -> int:
    args = parse_args(argv)
    logging.basicConfig(level=logging.WARNING if args.quiet else logging.INFO,
                        format="%(message)s", stream=sys.stdout)

    try:
        xous_root = resolve_xous_root(args.xous_root)
        app_index = None
        capture_fn = None
        outdir = None
        if args.app:
            app_index = args.app_index if args.app_index is not None else app_menu_index(xous_root, args.app)
            capture_fn = load_plugin(args.script) if args.script else BUILTIN.get(args.app, capture_default)
            outdir = os.path.abspath(args.screenshots or os.path.join("screenshots", args.app))
            os.makedirs(outdir, exist_ok=True)
        if args.init:
            reset_flash(xous_root)
    except (ConfigError, OSError) as e:
        log.error("error: %s", e)
        return EXIT_CONFIG

    log_path = args.renode_log or os.path.join(tempfile.gettempdir(), f"renode-{args.port}.log")
    failures = 0
    try:
        with RenodeSession(xous_root, port=args.port, log_path=log_path) as session:
            ctl = session.ctl
            boot_wait = args.boot_wait if args.boot_wait is not None else (60 if args.init else 45)
            log.info("waiting %.0fs for boot", boot_wait)
            ctl.sleep(boot_wait)
            if args.init:
                ctl.init_pddb(args.pin)
            elif not ctl.unlock_pddb(args.pin):
                log.warning("PDDB unlock did not settle; continuing")
            if args.app:
                log.info("launching %s (menu index %d), output %s", args.app, app_index, outdir)
                ctl.launch_app(app_index)
                try:
                    failures = run_capture(capture_fn, ctl, outdir)
                except RenodeError:
                    raise
                except Exception:
                    log.exception("capture sequence raised")
                    failures = max(1, ctl.screenshot_failures)
    except ConfigError as e:
        log.error("error: %s", e)
        return EXIT_CONFIG
    except RenodeError as e:
        log.error("Renode error: %s (log: %s)", e, log_path)
        return EXIT_RENODE
    except KeyboardInterrupt:
        log.error("interrupted")
        return 130

    if failures:
        log.error("FAILED: %d capture failure(s)", failures)
        return EXIT_FAILURES
    log.info("done")
    return EXIT_OK


if __name__ == "__main__":
    sys.exit(main())
