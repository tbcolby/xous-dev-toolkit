#!/usr/bin/env python3
"""
USB serial log monitor for a Precursor running Xous.

Setup on the device: open shellchat and type `usb console`.

    ./usb_log_monitor.py                     # auto-detect device
    ./usb_log_monitor.py /dev/cu.usbmodem1   # explicit device
    ./usb_log_monitor.py --list              # list candidate devices (no pyserial needed)
    ./usb_log_monitor.py --filter othello    # show only matching lines
    ./usb_log_monitor.py --save log.txt      # append ALL lines (unfiltered) to a file

Requires pyserial (`pip install pyserial`, or the `serial` extra) except for --list.

Key codes as seen by apps (services/keyboard/src/mappings/qwerty.rs):
    F1..F4 = 0x11..0x14 on hardware and in Renode (same matrix positions)
    Arrows: Up=0x2191 Down=0x2193 Left=0x2190 Right=0x2192, Menu=0x2234
    Enter=0x0D (CR), Space=0x20, Backspace=0x08
"""

from __future__ import annotations

import argparse
import codecs
import glob
import re
import sys
import time
from datetime import datetime

COLORS = {
    "ERROR": "\033[91m",
    "WARN": "\033[93m",
    "INFO": "\033[92m",
    "DEBUG": "\033[94m",
    "TRACE": "\033[90m",
    "RESET": "\033[0m",
    "BOLD": "\033[1m",
    "DIM": "\033[2m",
}

# Xous log format: "LEVEL:module: message (file:line)"
LOG_PATTERN = re.compile(r"^(ERROR|WARN|INFO|DEBUG|TRACE):([^:]+): (.+) \(([^)]+)\)$")

KEY_CODES = {
    0x0011: "F1",
    0x0012: "F2",
    0x0013: "F3",
    0x0014: "F4",
    0x2191: "Up",
    0x2193: "Down",
    0x2190: "Left",
    0x2192: "Right",
    0x2234: "Menu",
    0x000D: "Enter",
    0x000A: "LF",
    0x0008: "Backspace",
    0x0020: "Space",
}

SETUP_HINT = """To enable USB serial on Precursor:
  1. Open shellchat on the device
  2. Type: usb console
  3. Reconnect the USB cable if needed"""


def device_patterns(platform: str = sys.platform) -> list:
    if platform == "darwin":
        # cu.* is the callout node; tty.* can block waiting for carrier.
        return ["/dev/cu.usbmodem*"]
    return ["/dev/ttyACM*", "/dev/ttyUSB*"]


def find_precursor_device(platform: str = sys.platform) -> list:
    devices = []
    for pattern in device_patterns(platform):
        devices.extend(sorted(glob.glob(pattern)))
    return devices


def import_serial():
    try:
        import serial  # type: ignore
    except ImportError:
        sys.stderr.write(
            "error: pyserial is required: pip install pyserial  (or pip install '.[serial]')\n"
        )
        return None
    return serial


def colorize_log(line: str, use_colors: bool = True) -> str:
    if not use_colors:
        return line
    m = LOG_PATTERN.match(line.strip())
    if not m:
        return line
    level, module, message, location = m.groups()
    c, r = COLORS.get(level, ""), COLORS["RESET"]
    return f"{c}{level}{r}:{COLORS['BOLD']}{module}{r}: {message} {COLORS['DIM']}({location}){r}"


def decode_key_in_log(line: str) -> str:
    m = re.search(r"key[=\s]+0x([0-9a-fA-F]+)", line)
    if m:
        name = KEY_CODES.get(int(m.group(1), 16))
        if name:
            return f"{line}  <- {name}"
    return line


class LineSplitter:
    """Incremental UTF-8 decode + line split (multibyte chars may span reads)."""

    def __init__(self):
        self._decoder = codecs.getincrementaldecoder("utf-8")(errors="replace")
        self._buf = ""

    def feed(self, data: bytes) -> list:
        self._buf += self._decoder.decode(data)
        *lines, self._buf = self._buf.split("\n")
        return [ln.rstrip("\r") for ln in lines]


def monitor_serial(device, filter_keyword=None, save_file=None, use_colors=True, baud=115200) -> int:
    serial = import_serial()
    if serial is None:
        return 1
    print(f"Connecting to {device} at {baud} baud... (Ctrl+C to exit)")
    print("-" * 60)
    try:
        ser = serial.Serial(device, baud, timeout=0.1)
    except (serial.SerialException, OSError) as e:
        print(f"Error opening {device}: {e}\n\n{SETUP_HINT}")
        return 1

    save_fh = open(save_file, "a", encoding="utf-8") if save_file else None
    if save_fh:
        save_fh.write(f"\n=== Log session started: {datetime.now().isoformat()} ===\n")
    splitter = LineSplitter()
    needle = filter_keyword.lower() if filter_keyword else None
    rc = 0
    try:
        while True:
            try:
                data = ser.read(1024)
            except (serial.SerialException, OSError):
                print("\nSerial connection lost. Reconnecting...")
                ser.close()
                time.sleep(1)
                try:
                    ser = serial.Serial(device, baud, timeout=0.1)
                    print("Reconnected.")
                    continue
                except (serial.SerialException, OSError) as e:
                    print(f"Reconnection failed ({e}). Exiting.")
                    rc = 1
                    break
            if not data:
                continue
            for line in splitter.feed(data):
                ts = datetime.now().strftime("%H:%M:%S.%f")[:-3]
                if save_fh:  # unfiltered
                    save_fh.write(f"[{ts}] {line}\n")
                    save_fh.flush()
                if needle and needle not in line.lower():
                    continue
                print(f"[{ts}] {colorize_log(decode_key_in_log(line), use_colors)}")
    except KeyboardInterrupt:
        print("\nMonitoring stopped.")
    finally:
        ser.close()
        if save_fh:
            save_fh.write(f"=== Log session ended: {datetime.now().isoformat()} ===\n")
            save_fh.close()
            print(f"Logs saved to: {save_file}")
    return rc


def main(argv=None) -> int:
    p = argparse.ArgumentParser(
        description="Monitor USB serial logs from a Precursor device",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="Key codes" + __doc__.split("Key codes", 1)[1],
    )
    p.add_argument("device", nargs="?", help="serial device (auto-detect if omitted)")
    p.add_argument("--list", "-l", action="store_true", help="list candidate devices")
    p.add_argument("--filter", "-f", help="only display lines containing KEYWORD (case-insensitive)")
    p.add_argument("--save", "-s", help="append all lines (unfiltered) to FILE")
    p.add_argument("--no-color", action="store_true", help="disable colored output")
    p.add_argument("--baud", "-b", type=int, default=115200, help="baud rate (default: 115200)")
    args = p.parse_args(argv)

    devices = find_precursor_device()
    if args.list:
        if devices:
            print("Available USB serial devices:")
            for d in devices:
                print(f"  {d}")
        else:
            print(f"No USB serial devices found.\n\n{SETUP_HINT}")
        return 0

    device = args.device
    if not device:
        if not devices:
            print(f"No Precursor device found.\n\n{SETUP_HINT}\n\nOr pass the device path explicitly.")
            return 1
        device = devices[0]
        if len(devices) > 1:
            print(f"Multiple devices found, using {device} (others: {', '.join(devices[1:])})")

    return monitor_serial(device, filter_keyword=args.filter, save_file=args.save,
                          use_colors=not args.no_color, baud=args.baud)


if __name__ == "__main__":
    sys.exit(main())
