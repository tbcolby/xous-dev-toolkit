#!/usr/bin/env python3
"""
Renode automation for Precursor/Xous apps.

Handles PDDB initialization, app navigation, and screenshot capture.
Solves the keyboard hold timing problem using timed_key (pause→RunFor→release).

Usage:
    # Full init (blank flash) + capture flashcards screenshots:
    python3 renode_capture.py --init --app flashcards

    # Quick capture (PDDB already formatted with PIN 'a'):
    python3 renode_capture.py --app flashcards

    # Just init PDDB (no app launch):
    python3 renode_capture.py --init

    # Custom app with custom screenshot sequence:
    python3 renode_capture.py --app myapp --script my_captures.py
"""

import socket
import time
import re
import base64
import sys
import os
import subprocess
import argparse

MONITOR_PORT = 4567
XOUS_ROOT = "/Volumes/PlexLaCie/Dev/Precursor/xous-core"
RENODE = "/Applications/Renode.app/Contents/MacOS/renode"
DEFAULT_PIN = "a"


class RenodeController:
    """
    Controls Renode emulator via telnet monitor.

    Key methods:
    - timed_key(key): Press/release with exactly 1ms emulated hold time.
      Solves the 500ms hold threshold issue by using pause→RunFor→release.
    - inject_line(text): Inject characters + CR. Bypasses hold timing entirely.
      Use for text input (PIN entry, etc).
    - screenshot(path): Capture LCD as PNG via iTerm2 inline image protocol.
    """

    def __init__(self, port=MONITOR_PORT):
        self.port = port
        self.sock = None

    def connect(self):
        self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.sock.connect(('localhost', self.port))
        self.sock.settimeout(10)
        time.sleep(0.5)
        self._drain()
        self._send('mach set "SoC"')
        time.sleep(0.3)
        self._drain()

    def _send(self, cmd):
        self.sock.sendall((cmd + '\n').encode())

    def _drain(self):
        try:
            self.sock.recv(65536)
        except socket.timeout:
            pass

    def timed_key(self, key, hold_ms=1, after=1.0):
        """
        Press key with exactly hold_ms of emulated time between press and release.

        This solves the Renode keyboard hold timing issue:
        - Xous keyboard service has a 500ms hold threshold
        - Keys held > 500ms produce alternate characters (a→@) or are dropped
        - During CPU idle, emulated time advances faster than wall-clock
        - Solution: pause emulation, press key, RunFor exactly 1ms, release, resume

        Args:
            key: Renode scancode name (A, Return, Home, Down, Space, etc.)
            hold_ms: Emulated hold time in ms (default 1, must be < 500)
            after: Wall-clock seconds to wait after key for system to process
        """
        self._send('pause')
        time.sleep(0.2)
        self._send(f'sysbus.keyboard Press {key}')
        time.sleep(0.1)
        self._send(f'emulation RunFor "0:0:0.{hold_ms:03d}"')
        time.sleep(0.3)
        self._send(f'sysbus.keyboard Release {key}')
        time.sleep(0.1)
        self._send('start')
        time.sleep(after)

    def inject_line(self, text):
        """
        Inject string + CR (0x0D) into keyboard buffer.

        Bypasses hold timing entirely - characters go directly into the
        keyboard peripheral's UART_CHAR register via the INJECT interrupt path.

        CR (0x0D) is interpreted as:
        - Submit/confirm in PIN dialogs and text inputs
        - Submit action in radio dialogs (when cursor is on [Okay] button)
        - "Press any key" dismissal in notifications

        Args:
            text: String to inject (empty string = just send CR)
        """
        self._send(f'sysbus.keyboard InjectLine "{text}"')
        time.sleep(0.5)

    def screenshot(self, filepath):
        """Capture LCD state as PNG file. Returns file size or 0 on failure."""
        self._send('sysbus.memlcd TakeScreenshot')
        time.sleep(4.0)
        resp = b''
        while True:
            try:
                chunk = self.sock.recv(65536)
                if not chunk:
                    break
                resp += chunk
            except socket.timeout:
                break
        resp_str = resp.decode('utf-8', errors='replace')
        match = re.search(r'inline=1:([A-Za-z0-9+/=\s]+)', resp_str)
        if match:
            b64 = re.sub(r'\s', '', match.group(1))
            png = base64.b64decode(b64)
            os.makedirs(os.path.dirname(filepath), exist_ok=True)
            with open(filepath, 'wb') as f:
                f.write(png)
            print(f"  [{os.path.basename(filepath)}] {len(png)} bytes", flush=True)
            return len(png)
        print(f"  [{os.path.basename(filepath)}] FAILED", flush=True)
        return 0

    def quit(self):
        try:
            self._send('quit')
            time.sleep(1)
            self.sock.close()
        except:
            pass

    # === High-level sequences ===

    def confirm_radio_dialog(self):
        """
        Confirm a radio dialog by navigating to [Okay] button and pressing CR.

        Radio dialog layout: Radio1(idx=0), Radio2(idx=1), [Okay] button(idx=2).
        Submit only fires when cursor is at button (index >= items.len()).
        """
        self.timed_key('Down', after=1.5)
        self.timed_key('Down', after=1.5)
        self.inject_line("")  # CR at [Okay] button position

    def init_pddb(self, pin=DEFAULT_PIN):
        """
        Full PDDB initialization from blank flash.

        Sequence:
        1. Confirm format dialog (radio: Down, Down, CR)
        2. Enter PIN (InjectLine)
        3. Dismiss "press any key" notification (InjectLine CR)
        4. Confirm PIN (InjectLine)
        5. Wait for format (~6 min in Renode)
        6. Unlock with PIN (InjectLine)
        7. Wait for mount
        """
        print("[PDDB] Confirming format dialog...", flush=True)
        self.confirm_radio_dialog()
        time.sleep(5)

        print(f"[PDDB] Setting PIN '{pin}'...", flush=True)
        time.sleep(5)
        self.inject_line(pin)
        time.sleep(5)

        print("[PDDB] Dismissing notification...", flush=True)
        time.sleep(5)
        self.inject_line("")
        time.sleep(5)

        print(f"[PDDB] Confirming PIN '{pin}'...", flush=True)
        time.sleep(5)
        self.inject_line(pin)
        time.sleep(5)

        print("[PDDB] Formatting (this takes ~6 minutes)...", flush=True)
        for i in range(15):
            time.sleep(60)
            print(f"  Format check {i+1}/15...", flush=True)
            # Could add screenshot check here to detect completion

        print(f"[PDDB] Unlocking with PIN '{pin}'...", flush=True)
        time.sleep(5)
        self.inject_line(pin)
        time.sleep(10)

        print("[PDDB] Waiting 45s for mount...", flush=True)
        time.sleep(45)
        print("[PDDB] Ready!", flush=True)

    def unlock_pddb(self, pin=DEFAULT_PIN):
        """Unlock already-formatted PDDB with known PIN."""
        print(f"[PDDB] Unlocking with PIN '{pin}'...", flush=True)
        self.inject_line(pin)
        time.sleep(10)
        print("[PDDB] Waiting 45s for mount...", flush=True)
        time.sleep(45)

    def launch_app(self, app_index=1):
        """
        Navigate main menu to launch an app by index.

        Menu structure:
        - Home opens main menu
        - Item 0: Sleep
        - Item 1: Switch to App...
        - Home selects current item

        App submenu:
        - Item 0: Shellchat (default)
        - Item 1+: Custom apps (alphabetical)

        Args:
            app_index: Position in app submenu (0=Shellchat, 1=first custom app)
        """
        print(f"[NAV] Opening menu...", flush=True)
        self.timed_key('Home', after=3.0)

        print(f"[NAV] Selecting 'Switch to App...'...", flush=True)
        self.timed_key('Down', after=1.0)  # Skip "Sleep"
        self.timed_key('Home', after=3.0)  # Select "Switch to App..."

        print(f"[NAV] Selecting app at index {app_index}...", flush=True)
        for _ in range(app_index):
            self.timed_key('Down', after=1.0)
        self.timed_key('Home', after=5.0)  # Launch app

        time.sleep(10)  # Wait for app initialization
        print("[NAV] App launched!", flush=True)


def start_renode(xous_root=XOUS_ROOT):
    """Start Renode in background, wait for monitor port."""
    print("Starting Renode...", flush=True)
    proc = subprocess.Popen(
        [RENODE, "--disable-xwt", "-P", str(MONITOR_PORT),
         "-e", f"path add @{xous_root}; i @emulation/xous-release.resc"],
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
    )
    for _ in range(30):
        try:
            s = socket.socket()
            s.settimeout(1)
            s.connect(('localhost', MONITOR_PORT))
            s.close()
            print("  Monitor ready", flush=True)
            return proc
        except:
            time.sleep(1)
    print("ERROR: Renode monitor not ready after 30s")
    sys.exit(1)


def reset_flash(xous_root=XOUS_ROOT):
    """Reset renode.bin to blank 128MB flash (all 0xFF)."""
    path = os.path.join(xous_root, "tools/pddb-images/renode.bin")
    print(f"Resetting flash to blank: {path}", flush=True)
    with open(path, 'wb') as f:
        chunk = b'\xff' * (1024 * 1024)
        for _ in range(128):
            f.write(chunk)


def capture_timers(ctl, screenshot_dir):
    """Capture all timers app screenshots."""
    def ss(name):
        return ctl.screenshot(os.path.join(screenshot_dir, name))

    def enter(after=3.0):
        """Send Enter key via inject_line (CR), which reliably produces '\\r'."""
        ctl.inject_line("")
        time.sleep(after)

    print("\n=== Timers Screenshots ===", flush=True)

    # 1. Mode select screen (initial state, cursor on Pomodoro)
    print("  mode_select...", flush=True)
    time.sleep(2)
    ss("mode_select.png")

    # 2. Pomodoro (cursor already on Pomodoro, Enter to select)
    print("  pomodoro...", flush=True)
    enter()  # Enter Pomodoro mode
    enter()  # Start timer
    time.sleep(3)
    ss("pomodoro.png")

    # 3. Settings (press 's' from Pomodoro)
    print("  settings...", flush=True)
    ctl.timed_key('S', after=3.0)
    ss("settings.png")
    ctl.timed_key('Q', after=3.0)  # Back to Pomodoro
    ctl.timed_key('Q', after=3.0)  # Back to mode select

    # 4. Stopwatch (Down once to Stopwatch, Enter to select)
    print("  stopwatch...", flush=True)
    ctl.timed_key('Down', after=2.0)
    enter()  # Enter Stopwatch mode
    enter()  # Start stopwatch
    time.sleep(3)
    ctl.timed_key('L', after=2.0)  # Lap 1
    time.sleep(2)
    ctl.timed_key('L', after=2.0)  # Lap 2
    time.sleep(1)
    ss("stopwatch.png")
    enter()  # Pause
    ctl.timed_key('Q', after=3.0)  # Back to mode select

    # 5. Countdown list (Down twice to Countdown, Enter)
    print("  countdown_list...", flush=True)
    ctl.timed_key('Down', after=1.0)
    ctl.timed_key('Down', after=1.0)
    enter()  # Enter Countdown mode
    ss("countdown_list.png")
    ctl.timed_key('Q', after=3.0)  # Back to mode select

    print("=== Done! ===", flush=True)


def capture_flashcards(ctl, screenshot_dir):
    """Capture all flashcards app screenshots."""
    def ss(name):
        return ctl.screenshot(os.path.join(screenshot_dir, name))

    print("\n=== Flashcards Screenshots ===", flush=True)

    ss("deck_list.png")

    print("  deck_menu...", flush=True)
    ctl.timed_key('M', after=3.0)
    ss("deck_menu.png")
    ctl.timed_key('Q', after=2.0)

    print("  question/answer...", flush=True)
    ctl.inject_line("")  # CR opens deck for review
    time.sleep(3)
    ss("question.png")
    ctl.timed_key('Space', after=2.0)
    ss("answer.png")
    ctl.timed_key('Q', after=2.0)

    print("  import_wait...", flush=True)
    ctl.timed_key('I', after=3.0)
    ss("import_wait.png")

    print("=== Done! ===", flush=True)


def capture_writer(ctl, screenshot_dir):
    """Capture all writer app screenshots."""
    def ss(name):
        return ctl.screenshot(os.path.join(screenshot_dir, name))

    def enter(after=3.0):
        ctl.inject_line("")
        time.sleep(after)

    def esc_cmd(key_name, after=3.0):
        """Send Esc followed by a command key."""
        ctl.timed_key('Escape', after=0.5)
        ctl.timed_key(key_name, after=after)

    print("\n=== Writer Screenshots ===", flush=True)

    # 1. Mode select screen (initial state)
    print("  mode_select...", flush=True)
    time.sleep(2)
    ss("mode_select.png")

    # 2. Enter Markdown Editor (cursor already on it, press Enter)
    print("  editor...", flush=True)
    enter()  # → DocList
    time.sleep(2)

    # 3. New document (press 'n')
    ctl.timed_key('N', after=3.0)
    time.sleep(2)

    # 4. Type some markdown content
    ctl.inject_line("# My Document")
    time.sleep(1)
    ctl.inject_line("")  # blank line
    time.sleep(0.5)
    ctl.inject_line("This is a **writer** app for Precursor.")
    time.sleep(0.5)
    ctl.inject_line("")
    time.sleep(0.5)
    ctl.inject_line("## Features")
    time.sleep(0.5)
    ctl.inject_line("")
    time.sleep(0.5)
    ctl.inject_line("- Markdown styling")
    time.sleep(0.5)
    ctl.inject_line("- Journal mode")
    time.sleep(0.5)
    ctl.inject_line("- Typewriter mode")
    time.sleep(0.5)
    ctl.inject_line("")
    time.sleep(0.5)
    ctl.inject_line("> Built for focused writing")
    time.sleep(0.5)
    ctl.inject_line("")
    time.sleep(0.5)
    ctl.inject_line("---")
    time.sleep(2)

    # 5. Screenshot editor in edit mode
    ss("editor_edit.png")

    # 6. Preview mode (Esc + p)
    print("  editor_preview...", flush=True)
    esc_cmd('P', after=3.0)
    ss("editor_preview.png")

    # 7. Back to edit, then export menu (Esc + e)
    print("  export_menu...", flush=True)
    esc_cmd('P', after=2.0)  # toggle back to edit
    esc_cmd('E', after=3.0)  # export menu
    ss("export_menu.png")

    # 8. Cancel export, back to doc list, back to mode select
    ctl.timed_key('Q', after=2.0)  # cancel export
    esc_cmd('Q', after=3.0)  # back to doc list
    ctl.timed_key('Q', after=3.0)  # back to mode select

    # 9. Journal mode (Down once, Enter)
    print("  journal...", flush=True)
    ctl.timed_key('Down', after=2.0)
    enter()  # Enter journal
    time.sleep(3)

    # Type some journal content
    ctl.inject_line("Today I started using the Writer app.")
    time.sleep(0.5)
    ctl.inject_line("It has markdown support and auto-saves.")
    time.sleep(0.5)
    ctl.inject_line("")
    time.sleep(0.5)
    ctl.inject_line("Planning to write more tomorrow.")
    time.sleep(2)
    ss("journal.png")

    # Back to mode select
    esc_cmd('Q', after=3.0)

    # 10. Typewriter mode (Down twice from top, Enter)
    print("  typewriter...", flush=True)
    ctl.timed_key('Down', after=1.0)
    ctl.timed_key('Down', after=1.0)
    enter()  # Enter typewriter
    time.sleep(2)

    # Type some freewriting content
    ctl.inject_line("The quick brown fox jumps over the lazy dog.")
    time.sleep(0.5)
    ctl.inject_line("Writing flows freely when you cannot edit.")
    time.sleep(0.5)
    ctl.inject_line("Just keep moving forward.")
    time.sleep(2)
    ss("typewriter.png")

    print("=== Done! ===", flush=True)


def main():
    parser = argparse.ArgumentParser(description="Renode automation for Precursor apps")
    parser.add_argument('--init', action='store_true', help='Initialize PDDB from blank flash')
    parser.add_argument('--app', type=str, help='App name to launch and capture')
    parser.add_argument('--pin', type=str, default=DEFAULT_PIN, help='PDDB PIN (default: a)')
    parser.add_argument('--app-index', type=int, default=1, help='App position in submenu (default: 1)')
    parser.add_argument('--screenshots', type=str, help='Screenshot output directory')
    parser.add_argument('--xous-root', type=str, default=XOUS_ROOT)
    args = parser.parse_args()

    if args.init:
        reset_flash(args.xous_root)

    proc = start_renode(args.xous_root)
    ctl = RenodeController()
    ctl.connect()
    ctl._send('start')
    time.sleep(1)

    # Boot wait
    boot_time = 60 if args.init else 45
    print(f"Waiting {boot_time}s for boot...", flush=True)
    time.sleep(boot_time)

    # PDDB setup
    if args.init:
        ctl.init_pddb(args.pin)
    else:
        ctl.unlock_pddb(args.pin)

    # App launch
    if args.app:
        ctl.launch_app(args.app_index)

        # App-specific captures
        screenshot_dir = args.screenshots or f"/Volumes/PlexLaCie/Dev/Precursor/precursor-{args.app}/screenshots"
        if args.app == 'flashcards':
            capture_flashcards(ctl, screenshot_dir)
        elif args.app == 'timers':
            capture_timers(ctl, screenshot_dir)
        elif args.app == 'writer':
            capture_writer(ctl, screenshot_dir)
        else:
            print(f"No capture sequence defined for '{args.app}'. Taking one screenshot.", flush=True)
            ctl.screenshot(os.path.join(screenshot_dir, "app.png"))

    ctl.quit()
    proc.terminate()
    try:
        proc.wait(timeout=5)
    except:
        proc.kill()


if __name__ == '__main__':
    main()
