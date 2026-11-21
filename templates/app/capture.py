"""Renode capture sequence for {{display}}.

Run from xous-dev-toolkit:
    python3 scripts/renode_capture.py --app {{name}} --script ../precursor-{{name}}/capture.py

Contract: run(ctl, outdir) -> number of failed steps. The app is already launched.
"""
import os


def run(ctl, outdir) -> int:
    failures = 0

    def shot(name):
        nonlocal failures
        if not ctl.try_screenshot(os.path.join(outdir, name)):
            failures += 1

    shot("01_list.png")
    ctl.timed_key("Down", after=1.0)
    ctl.timed_key("Down", after=1.0)
    shot("02_moved.png")
    ctl.inject_line("")  # Enter: +1
    ctl.sleep(1.0)
    shot("03_bumped.png")
    return failures
