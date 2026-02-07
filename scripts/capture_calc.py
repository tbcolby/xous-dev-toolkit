#!/usr/bin/env python3
"""
Standalone calculator capture script.

Uses renode_lib.py for core Renode control. This script provides a simpler
alternative to renode_capture.py for quick calculator-only testing.

Usage:
    python3 capture_calc.py [--screenshots DIR] [--xous-root PATH]
"""

import time
import os
import sys
import argparse

# Add scripts dir to path for import
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from renode_lib import RenodeController, start_renode, XOUS_ROOT, DEFAULT_PIN


def capture_calc_standalone(ctl, screenshot_dir):
    """Capture calculator screenshots using InjectString/InjectKey for fine control."""
    def ss(name):
        return ctl.screenshot(os.path.join(screenshot_dir, name))

    def enter(after=2.0):
        ctl.inject_line("")
        time.sleep(after)

    print("\n=== Calculator Standalone Capture ===", flush=True)

    # Initial state
    print("  [1] Initial state...", flush=True)
    time.sleep(3)
    ss("01_initial.png")

    # Expression: 2+3*4
    print("  [2] Expression: 2+3*4...", flush=True)
    ctl.inject_string("2+3*4")
    time.sleep(1)
    ss("02_expression.png")

    # Evaluate
    print("  [3] Evaluate...", flush=True)
    enter(after=2.0)
    ss("03_result.png")

    # sin(90)
    print("  [4] sin(90)...", flush=True)
    ctl.inject_string("sin(90)")
    time.sleep(1)
    enter(after=2.0)
    ss("04_sin90.png")

    # Toggle RPN mode
    print("  [5] RPN mode...", flush=True)
    ctl.inject_key('M')
    time.sleep(2)
    ss("05_rpn_mode.png")

    # RPN: 2 Enter 3 +
    print("  [6] RPN: 2 Enter 3 +...", flush=True)
    ctl.inject_key('2')
    time.sleep(0.5)
    enter(after=1.5)
    ctl.inject_key('3')
    time.sleep(0.5)
    ctl.inject_key('+')
    time.sleep(1)
    ss("06_rpn_result.png")

    print("=== Done! ===", flush=True)


def main():
    parser = argparse.ArgumentParser(description="Calculator capture for Renode")
    parser.add_argument('--screenshots', type=str,
                        default=os.path.join(XOUS_ROOT, "apps/calc/screenshots"),
                        help='Screenshot output directory')
    parser.add_argument('--xous-root', type=str, default=XOUS_ROOT)
    parser.add_argument('--app-index', type=int, default=2,
                        help='Calculator position in app submenu (default: 2)')
    args = parser.parse_args()

    os.makedirs(args.screenshots, exist_ok=True)

    proc = start_renode(args.xous_root)
    ctl = RenodeController()
    ctl.connect()
    ctl._send('start')
    time.sleep(1)

    try:
        print("Waiting 45s for boot...", flush=True)
        time.sleep(45)

        ctl.unlock_pddb()

        # Navigate to calculator
        ctl.launch_app(args.app_index)

        capture_calc_standalone(ctl, args.screenshots)
    finally:
        ctl.quit()
        proc.terminate()
        try:
            proc.wait(timeout=5)
        except Exception:
            proc.kill()


if __name__ == '__main__':
    main()
