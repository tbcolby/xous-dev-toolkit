#!/usr/bin/env python3
"""
Renode Interaction Toolkit for Precursor/Xous Development

Provides programmatic control of the Renode emulator for headless testing
on platforms where the Renode GUI (XWT) is unavailable (e.g., macOS ARM64).

Features:
- Screenshot capture from the emulated LCD display
- Keyboard input with hold-timing-safe key presses
- Full PDDB initialization automation
- App menu navigation

Usage:
    python3 renode_interact.py screenshot output.png
    python3 renode_interact.py init-pddb
    python3 renode_interact.py launch-app Flashcards
    python3 renode_interact.py press-key Home
"""

import socket
import time
import re
import base64
import sys
import os


class RenodeController:
    """Controls a Renode instance via its telnet monitor port."""

    def __init__(self, host='127.0.0.1', port=4567, machine='SoC'):
        self.host = host
        self.port = port
        self.machine = machine
        self.sock = None

    def connect(self):
        """Connect to the Renode telnet monitor."""
        self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.sock.connect((self.host, self.port))
        self.sock.settimeout(10)
        time.sleep(0.3)
        self._drain()
        # Set machine context
        self._send(f'mach set "{self.machine}"')
        time.sleep(0.3)
        self._drain()

    def disconnect(self):
        """Close the telnet connection."""
        if self.sock:
            self.sock.close()
            self.sock = None

    def _send(self, cmd):
        """Send a command to the monitor."""
        self.sock.sendall((cmd + '\n').encode())

    def _drain(self):
        """Read and discard pending data."""
        try:
            self.sock.recv(65536)
        except socket.timeout:
            pass

    def screenshot(self, filename):
        """
        Capture the current LCD display state as a PNG file.

        The memlcd peripheral returns an iTerm2 inline image protocol
        response containing base64-encoded PNG data.

        Returns: PNG file size in bytes, or 0 on failure.
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
            png_data = base64.b64decode(b64)
            with open(filename, 'wb') as f:
                f.write(png_data)
            return len(png_data)
        return 0

    def fast_key(self, key):
        """
        Press and release a key with minimal emulated time between events.

        CRITICAL: The Xous keyboard service has a 500ms hold threshold.
        Keys held longer produce the 'hold' character variant. Many
        navigation keys (arrows, Home, Space) have hold=None and will be
        SILENTLY DROPPED if held too long.

        This method sends Press and Release in a single socket write to
        minimize the emulated time gap between them.

        Args:
            key: Renode KeyScanCode name (e.g., 'Home', 'A', 'Down', 'Return')
        """
        self._send(f'sysbus.keyboard Press {key}\nsysbus.keyboard Release {key}')
        time.sleep(0.4)

    def type_text(self, text):
        """
        Type a string of characters using fast key presses.
        Only supports a-z (maps to scan codes A-Z).
        """
        for ch in text:
            if ch.isalpha():
                self.fast_key(ch.upper())
            elif ch == ' ':
                self.fast_key('Space')
            elif ch == '\n' or ch == '\r':
                self.fast_key('Return')
            time.sleep(0.1)

    def wait_for_change(self, reference_size, timeout=600, interval=60):
        """
        Monitor screenshots until the image size changes significantly.
        Used to detect when format/progress dialogs complete.

        Args:
            reference_size: Size of screenshots during the wait state
            timeout: Maximum seconds to wait
            interval: Seconds between checks

        Returns: True if change detected, False if timeout
        """
        elapsed = 0
        while elapsed < timeout:
            time.sleep(interval)
            elapsed += interval
            sz = self.screenshot('/tmp/_monitor.png')
            if sz < reference_size * 0.5 or sz > reference_size * 1.5:
                return True
        return False


def init_pddb(ctl, pin='a'):
    """
    Automate the full PDDB initialization sequence on a blank flash image.

    Sequence:
    1. Format dialog (Okay/Cancel) -> confirm Okay
    2. PIN entry -> type PIN and confirm
    3. "Press any key" notification -> dismiss
    4. PIN confirmation -> re-type PIN and confirm
    5. Format process (~5 minutes)

    Args:
        ctl: Connected RenodeController instance
        pin: The PIN to set (default: 'a', matches Renode keybox)
    """
    print('[PDDB] Navigating format dialog...', flush=True)
    # Radio buttons: Okay is selected by default, Down twice to reach [Okay] button
    ctl.fast_key('Down')
    time.sleep(0.3)
    ctl.fast_key('Down')
    time.sleep(0.3)
    ctl.fast_key('Home')  # confirm with menu key
    time.sleep(5.0)

    print(f'[PDDB] Entering PIN "{pin}" (first time)...', flush=True)
    time.sleep(3)
    ctl.type_text(pin)
    time.sleep(1.0)
    ctl.fast_key('Home')  # confirm
    time.sleep(5.0)

    print('[PDDB] Dismissing notification...', flush=True)
    time.sleep(3)
    ctl.fast_key('Return')
    time.sleep(5.0)

    print(f'[PDDB] Entering PIN "{pin}" (confirmation)...', flush=True)
    time.sleep(3)
    ctl.type_text(pin)
    time.sleep(1.0)
    ctl.fast_key('Home')  # confirm
    time.sleep(5.0)

    print('[PDDB] Format in progress (monitoring)...', flush=True)
    for i in range(10):
        time.sleep(60)
        sz = ctl.screenshot(f'/tmp/pddb_fmt_{i:02d}.png')
        if sz < 5000:
            print(f'[PDDB] Format complete at {(i+1)*60}s!', flush=True)
            return True
        print(f'[PDDB] Still formatting... ({(i+1)*60}s elapsed)', flush=True)

    print('[PDDB] WARNING: Format may not have completed in time', flush=True)
    return False


def launch_app(ctl, app_name='Flashcards', position=1):
    """
    Navigate the main menu to launch an app.

    Args:
        ctl: Connected RenodeController instance
        app_name: Name of the app (for logging)
        position: Position in the app submenu (0=first/Shellchat, 1=second, etc.)
    """
    print(f'[NAV] Opening main menu...', flush=True)
    ctl.fast_key('Home')
    time.sleep(3.0)

    print(f'[NAV] Selecting "Switch to App..."...', flush=True)
    ctl.fast_key('Down')  # Move from Sleep to Switch to App
    time.sleep(0.5)
    ctl.fast_key('Home')  # Select
    time.sleep(3.0)

    print(f'[NAV] Selecting {app_name} (position {position})...', flush=True)
    for _ in range(position):
        ctl.fast_key('Down')
        time.sleep(0.5)
    ctl.fast_key('Home')  # Select the app
    time.sleep(3.0)

    sz = ctl.screenshot(f'/tmp/{app_name.lower()}_launched.png')
    print(f'[NAV] {app_name} launched! Screenshot: {sz} bytes', flush=True)


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)

    cmd = sys.argv[1]
    ctl = RenodeController()

    if cmd == 'screenshot':
        filename = sys.argv[2] if len(sys.argv) > 2 else '/tmp/screenshot.png'
        ctl.connect()
        sz = ctl.screenshot(filename)
        ctl.disconnect()
        if sz:
            print(f'Saved {filename} ({sz} bytes)')
        else:
            print('Failed to capture screenshot')
            sys.exit(1)

    elif cmd == 'init-pddb':
        pin = sys.argv[2] if len(sys.argv) > 2 else 'a'
        ctl.connect()
        success = init_pddb(ctl, pin)
        ctl.disconnect()
        if not success:
            sys.exit(1)

    elif cmd == 'launch-app':
        app_name = sys.argv[2] if len(sys.argv) > 2 else 'Flashcards'
        position = int(sys.argv[3]) if len(sys.argv) > 3 else 1
        ctl.connect()
        launch_app(ctl, app_name, position)
        ctl.disconnect()

    elif cmd == 'press-key':
        key = sys.argv[2]
        ctl.connect()
        ctl.fast_key(key)
        ctl.disconnect()
        print(f'Pressed {key}')

    elif cmd == 'full-init':
        # Full sequence: wait for boot, init PDDB, launch app
        boot_wait = int(sys.argv[2]) if len(sys.argv) > 2 else 90
        print(f'[BOOT] Waiting {boot_wait}s for system boot...', flush=True)
        time.sleep(boot_wait)

        ctl.connect()
        ctl.screenshot('/tmp/boot_state.png')
        init_pddb(ctl, 'a')
        time.sleep(10)
        launch_app(ctl, 'Flashcards', 1)
        ctl.disconnect()

    else:
        print(f'Unknown command: {cmd}')
        print(__doc__)
        sys.exit(1)


if __name__ == '__main__':
    main()
