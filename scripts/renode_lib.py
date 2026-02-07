#!/usr/bin/env python3
"""
Shared Renode automation library for Precursor/Xous development.

Provides the RenodeController class for controlling a Renode emulator instance
via its telnet monitor port. Handles keyboard timing, text injection, screenshot
capture, PDDB initialization, and app navigation.

This module consolidates previously duplicated code from renode_capture.py,
capture_calc.py, and renode_interact.py into a single reusable library.

Usage as library:
    from renode_lib import RenodeController, start_renode, reset_flash

    proc = start_renode()
    ctl = RenodeController()
    ctl.connect()
    ctl.timed_key('Home', after=3.0)
    ctl.screenshot('/tmp/screen.png')

Configuration via environment variables:
    RENODE_PATH     Path to Renode binary (default: /Applications/Renode.app/Contents/MacOS/renode)
    XOUS_ROOT       Path to xous-core checkout (default: /Volumes/PlexLaCie/Dev/Precursor/xous-core)
    RENODE_PORT     Monitor telnet port (default: 4567)
"""

import socket
import time
import re
import base64
import hashlib
import os
import subprocess
import sys

# Configurable via environment variables
MONITOR_PORT = int(os.environ.get('RENODE_PORT', '4567'))
XOUS_ROOT = os.environ.get('XOUS_ROOT', '/Volumes/PlexLaCie/Dev/Precursor/xous-core')
RENODE = os.environ.get('RENODE_PATH', '/Applications/Renode.app/Contents/MacOS/renode')
DEFAULT_PIN = 'a'

# PNG file signature (first 8 bytes)
PNG_SIGNATURE = b'\x89PNG\r\n\x1a\n'


class RenodeController:
    """
    Controls Renode emulator via telnet monitor.

    Key methods:
    - timed_key(key): Press/release with exactly 1ms emulated hold time.
      Solves the 500ms hold threshold issue by using pause->RunFor->release.
    - inject_line(text): Inject characters + CR. Bypasses hold timing entirely.
      Use for text input (PIN entry, etc).
    - inject_string(text): Inject characters without CR. For partial input.
    - inject_key(char): Inject a single character by ASCII code.
    - screenshot(path): Capture LCD as PNG via iTerm2 inline image protocol.
    - wait_for_screen_change(): Wait until the screen visually changes.
    """

    def __init__(self, port=None):
        self.port = port or MONITOR_PORT
        self.sock = None
        self._last_screenshot_hash = None

    def connect(self, retries=3, backoff=2.0):
        """
        Connect to the Renode telnet monitor with retry logic.

        Args:
            retries: Number of connection attempts (default: 3)
            backoff: Seconds between retry attempts (default: 2.0)

        Raises:
            RuntimeError: If all connection attempts fail
        """
        last_error = None
        for attempt in range(retries):
            try:
                self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
                self.sock.settimeout(10)
                self.sock.connect(('localhost', self.port))
                time.sleep(0.5)
                self._drain()
                self._send('mach set "SoC"')
                time.sleep(0.3)
                self._drain()
                return  # Success
            except (socket.error, OSError) as e:
                last_error = e
                if self.sock:
                    try:
                        self.sock.close()
                    except OSError:
                        pass
                    self.sock = None
                if attempt < retries - 1:
                    print(f"  Connection attempt {attempt + 1}/{retries} failed: {e}", flush=True)
                    time.sleep(backoff)
        raise RuntimeError(
            f"Failed to connect to Renode monitor on port {self.port} "
            f"after {retries} attempts: {last_error}"
        )

    def _send(self, cmd):
        """Send a command to the monitor."""
        self.sock.sendall((cmd + '\n').encode())

    def _drain(self):
        """Read and discard pending data from socket."""
        try:
            self.sock.recv(65536)
        except socket.timeout:
            pass

    def timed_key(self, key, hold_ms=1, after=1.0):
        """
        Press key with exactly hold_ms of emulated time between press and release.

        This solves the Renode keyboard hold timing issue:
        - Xous keyboard service has a 500ms hold threshold
        - Keys held > 500ms produce alternate characters (a->@) or are dropped
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

    def inject_string(self, text):
        """
        Inject string WITHOUT trailing CR. Use for partial text input
        where you don't want to submit yet.

        Args:
            text: String to inject
        """
        self._send(f'sysbus.keyboard InjectString "{text}"')
        time.sleep(0.5)

    def inject_key(self, char):
        """
        Inject a single character by its ASCII code.

        Useful for sending specific characters that are hard to type via
        scan codes (operators, punctuation, etc).

        Args:
            char: Single character to inject (e.g., '+', 'm', '2')
        """
        self._send(f'sysbus.keyboard InjectKey {ord(char)}')
        time.sleep(0.3)

    def screenshot(self, filepath, retries=2):
        """
        Capture LCD state as PNG file with validation and retry.

        Returns file size in bytes, or 0 on failure.

        Args:
            filepath: Output PNG file path
            retries: Number of attempts on failure (default: 2)
        """
        for attempt in range(retries):
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

            # Pattern 1: iTerm2 inline image protocol
            match = re.search(r'inline=1:([A-Za-z0-9+/=\s]+)', resp_str)
            if match:
                b64 = re.sub(r'\s', '', match.group(1))
                try:
                    png = base64.b64decode(b64)
                except Exception:
                    png = b''

                # Validate PNG signature
                if png[:8] == PNG_SIGNATURE:
                    os.makedirs(os.path.dirname(filepath) or '.', exist_ok=True)
                    with open(filepath, 'wb') as f:
                        f.write(png)
                    self._last_screenshot_hash = hashlib.md5(png).hexdigest()
                    print(f"  [{os.path.basename(filepath)}] {len(png)} bytes", flush=True)
                    return len(png)

            # Pattern 2: Raw base64 fallback (look for PNG header in decoded)
            b64_match = re.search(r'([A-Za-z0-9+/]{100,}={0,2})', resp_str)
            if b64_match:
                try:
                    b64_data = b64_match.group(1)
                    png = base64.b64decode(b64_data)
                    if png[:8] == PNG_SIGNATURE:
                        os.makedirs(os.path.dirname(filepath) or '.', exist_ok=True)
                        with open(filepath, 'wb') as f:
                            f.write(png)
                        self._last_screenshot_hash = hashlib.md5(png).hexdigest()
                        print(f"  [{os.path.basename(filepath)}] {len(png)} bytes (fallback)", flush=True)
                        return len(png)
                except Exception:
                    pass

            if attempt < retries - 1:
                print(f"  [{os.path.basename(filepath)}] attempt {attempt + 1} failed, retrying...", flush=True)
                time.sleep(1)

        print(f"  [{os.path.basename(filepath)}] FAILED after {retries} attempts", flush=True)
        return 0

    def screenshot_hash(self, tmp_path='/tmp/_renode_hash_check.png'):
        """
        Take a screenshot and return its MD5 hash without saving permanently.
        Useful for detecting screen state changes.
        """
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
            try:
                png = base64.b64decode(b64)
                if png[:8] == PNG_SIGNATURE:
                    return hashlib.md5(png).hexdigest()
            except Exception:
                pass
        return None

    def wait_for_screen_change(self, timeout=60, interval=5):
        """
        Wait until the screen content changes from the last screenshot.

        Takes a reference screenshot, then polls until the screen differs.
        Useful for detecting when dialogs dismiss, formats complete, etc.

        Args:
            timeout: Maximum seconds to wait (default: 60)
            interval: Seconds between checks (default: 5)

        Returns:
            True if screen changed, False if timeout
        """
        ref_hash = self.screenshot_hash()
        if ref_hash is None:
            return False

        elapsed = 0
        while elapsed < timeout:
            time.sleep(interval)
            elapsed += interval
            current_hash = self.screenshot_hash()
            if current_hash and current_hash != ref_hash:
                return True

        return False

    def quit(self):
        """Send quit command and close socket."""
        try:
            self._send('quit')
            time.sleep(1)
            self.sock.close()
        except (OSError, AttributeError):
            pass
        self.sock = None

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
        print("[NAV] Opening menu...", flush=True)
        self.timed_key('Home', after=3.0)

        print("[NAV] Selecting 'Switch to App...'...", flush=True)
        self.timed_key('Down', after=1.0)  # Skip "Sleep"
        self.timed_key('Home', after=3.0)  # Select "Switch to App..."

        print(f"[NAV] Selecting app at index {app_index}...", flush=True)
        for _ in range(app_index):
            self.timed_key('Down', after=1.0)
        self.timed_key('Home', after=5.0)  # Launch app

        time.sleep(10)  # Wait for app initialization
        print("[NAV] App launched!", flush=True)


def start_renode(xous_root=None):
    """
    Start Renode in background and wait for monitor port to be ready.

    Args:
        xous_root: Path to xous-core (default: from env or XOUS_ROOT constant)

    Returns:
        subprocess.Popen instance for the Renode process
    """
    xous_root = xous_root or XOUS_ROOT
    renode_path = RENODE

    if not os.path.exists(renode_path):
        print(f"ERROR: Renode not found at {renode_path}")
        print("Set RENODE_PATH environment variable to your Renode binary.")
        sys.exit(1)

    print("Starting Renode...", flush=True)
    proc = subprocess.Popen(
        [renode_path, "--disable-xwt", "-P", str(MONITOR_PORT),
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
        except (socket.error, OSError):
            time.sleep(1)
    print("ERROR: Renode monitor not ready after 30s")
    proc.kill()
    sys.exit(1)


def reset_flash(xous_root=None):
    """
    Reset renode.bin to blank 128MB flash (all 0xFF).

    Args:
        xous_root: Path to xous-core (default: from env or XOUS_ROOT constant)
    """
    xous_root = xous_root or XOUS_ROOT
    path = os.path.join(xous_root, "tools/pddb-images/renode.bin")
    print(f"Resetting flash to blank: {path}", flush=True)
    with open(path, 'wb') as f:
        chunk = b'\xff' * (1024 * 1024)
        for _ in range(128):
            f.write(chunk)
