#!/usr/bin/env python3
"""
USB Serial Log Monitor for Precursor Development

This script monitors the USB serial console from a Precursor device running Xous.
It provides real-time log output for debugging apps during development.

SETUP (on the Precursor device):
1. Open the shellchat app
2. Type: usb console
3. This switches USB to serial mode and enables log streaming

USAGE:
    ./usb_log_monitor.py                    # Auto-detect device
    ./usb_log_monitor.py /dev/tty.usbmodem* # Specify device
    ./usb_log_monitor.py --list             # List available devices
    ./usb_log_monitor.py --filter othello   # Filter logs by keyword
    ./usb_log_monitor.py --save log.txt     # Save logs to file

Key codes reference (for debugging key handling):
    Hardware F1=0x0011, F2=0x0012, F3=0x0013, F4=0x0014
    Renode   F1=0xF001, F2=0xF002, F3=0xF003, F4=0xF004
    Arrows: Up=0x2191, Down=0x2193, Left=0x2190, Right=0x2192
    Enter=0x000D (CR), Space=0x0020
"""

import argparse
import glob
import os
import re
import select
import serial
import sys
import time
from datetime import datetime

# ANSI colors for log levels
COLORS = {
    'ERROR': '\033[91m',  # Red
    'WARN': '\033[93m',   # Yellow
    'INFO': '\033[92m',   # Green
    'DEBUG': '\033[94m',  # Blue
    'TRACE': '\033[90m',  # Gray
    'RESET': '\033[0m',
    'BOLD': '\033[1m',
    'DIM': '\033[2m',
}

# Regex to parse Xous log format: "LEVEL:module: message (file:line)"
LOG_PATTERN = re.compile(r'^(ERROR|WARN|INFO|DEBUG|TRACE):([^:]+): (.+) \(([^)]+)\)$')

# Key code mappings for reference
KEY_CODES = {
    0x0011: 'F1 (hw)',
    0x0012: 'F2 (hw)',
    0x0013: 'F3 (hw)',
    0x0014: 'F4 (hw)',
    0xF001: 'F1 (renode)',
    0xF002: 'F2 (renode)',
    0xF003: 'F3 (renode)',
    0xF004: 'F4 (renode)',
    0x2191: 'Up',
    0x2193: 'Down',
    0x2190: 'Left',
    0x2192: 'Right',
    0x000D: 'Enter',
    0x000A: 'LF',
    0x0020: 'Space',
    0x001B: 'Escape',
}


def find_precursor_device():
    """Find the Precursor USB serial device."""
    # Common patterns for Precursor on different platforms
    patterns = [
        '/dev/tty.usbmodem*',      # macOS
        '/dev/ttyACM*',            # Linux
        '/dev/ttyUSB*',            # Linux alternative
        '/dev/cu.usbmodem*',       # macOS alternative
    ]

    devices = []
    for pattern in patterns:
        devices.extend(glob.glob(pattern))

    return devices


def list_devices():
    """List all available USB serial devices."""
    devices = find_precursor_device()
    if not devices:
        print("No USB serial devices found.")
        print("\nTo enable USB serial on Precursor:")
        print("  1. Open shellchat on the device")
        print("  2. Type: usb console")
        print("  3. Reconnect USB cable if needed")
        return

    print("Available USB serial devices:")
    for dev in devices:
        print(f"  {dev}")


def colorize_log(line, use_colors=True):
    """Apply colors to log line based on level."""
    if not use_colors:
        return line

    match = LOG_PATTERN.match(line.strip())
    if match:
        level, module, message, location = match.groups()
        color = COLORS.get(level, '')
        reset = COLORS['RESET']
        dim = COLORS['DIM']
        return f"{color}{level}{reset}:{COLORS['BOLD']}{module}{reset}: {message} {dim}({location}){reset}"

    return line


def decode_key_in_log(line):
    """If line contains a key code, annotate it."""
    # Look for patterns like "key 0x0012" or "key=0x0012"
    key_match = re.search(r'key[=\s]+0x([0-9a-fA-F]+)', line)
    if key_match:
        code = int(key_match.group(1), 16)
        if code in KEY_CODES:
            return f"{line}  <- {KEY_CODES[code]}"
    return line


def monitor_serial(device, filter_keyword=None, save_file=None, use_colors=True, baud=115200):
    """Monitor the serial port and display/save logs."""
    print(f"Connecting to {device} at {baud} baud...")
    print("Press Ctrl+C to exit\n")
    print("-" * 60)

    try:
        ser = serial.Serial(device, baud, timeout=0.1)
    except serial.SerialException as e:
        print(f"Error opening {device}: {e}")
        print("\nTroubleshooting:")
        print("  - Is the device connected?")
        print("  - Did you run 'usb console' on the Precursor?")
        print("  - Try unplugging and replugging the USB cable")
        return 1

    save_fh = None
    if save_file:
        save_fh = open(save_file, 'a')
        save_fh.write(f"\n=== Log session started: {datetime.now().isoformat()} ===\n")

    line_buffer = ""
    try:
        while True:
            try:
                data = ser.read(1024)
                if data:
                    text = data.decode('utf-8', errors='replace')
                    line_buffer += text

                    # Process complete lines
                    while '\n' in line_buffer:
                        line, line_buffer = line_buffer.split('\n', 1)
                        line = line.rstrip('\r')

                        # Apply filter if specified
                        if filter_keyword and filter_keyword.lower() not in line.lower():
                            continue

                        # Decode key codes if present
                        line = decode_key_in_log(line)

                        # Colorize and print
                        display_line = colorize_log(line, use_colors)
                        timestamp = datetime.now().strftime('%H:%M:%S.%f')[:-3]
                        print(f"[{timestamp}] {display_line}")

                        # Save to file if specified
                        if save_fh:
                            save_fh.write(f"[{timestamp}] {line}\n")
                            save_fh.flush()

            except serial.SerialException:
                print("\nSerial connection lost. Attempting to reconnect...")
                ser.close()
                time.sleep(1)
                try:
                    ser = serial.Serial(device, baud, timeout=0.1)
                    print("Reconnected!")
                except:
                    print("Reconnection failed. Exiting.")
                    break

    except KeyboardInterrupt:
        print("\n\nMonitoring stopped.")
    finally:
        ser.close()
        if save_fh:
            save_fh.write(f"=== Log session ended: {datetime.now().isoformat()} ===\n")
            save_fh.close()
            print(f"Logs saved to: {save_file}")

    return 0


def main():
    parser = argparse.ArgumentParser(
        description='Monitor USB serial logs from Precursor device',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
    %(prog)s                          # Auto-detect and monitor
    %(prog)s --list                   # List available devices
    %(prog)s /dev/tty.usbmodemXXX     # Monitor specific device
    %(prog)s --filter othello         # Only show logs containing 'othello'
    %(prog)s --save debug.log         # Save logs to file
    %(prog)s --no-color               # Disable color output

Setup on Precursor:
    1. Open shellchat
    2. Type: usb console
    3. Run this script on your computer

Key Code Reference:
    F1=0x0011/0xF001  F2=0x0012/0xF002  F3=0x0013/0xF003  F4=0x0014/0xF004
    Up=0x2191  Down=0x2193  Left=0x2190  Right=0x2192
    Enter=0x000D  Space=0x0020  Escape=0x001B
"""
    )

    parser.add_argument('device', nargs='?', help='Serial device path (auto-detect if not specified)')
    parser.add_argument('--list', '-l', action='store_true', help='List available devices')
    parser.add_argument('--filter', '-f', help='Filter logs by keyword')
    parser.add_argument('--save', '-s', help='Save logs to file')
    parser.add_argument('--no-color', action='store_true', help='Disable colored output')
    parser.add_argument('--baud', '-b', type=int, default=115200, help='Baud rate (default: 115200)')

    args = parser.parse_args()

    if args.list:
        list_devices()
        return 0

    device = args.device
    if not device:
        devices = find_precursor_device()
        if not devices:
            print("No Precursor device found.")
            print("\nTo enable USB serial on Precursor:")
            print("  1. Open shellchat on the device")
            print("  2. Type: usb console")
            print("  3. Reconnect USB cable if needed")
            print("\nOr specify device manually: ./usb_log_monitor.py /dev/ttyXXX")
            return 1
        device = devices[0]
        if len(devices) > 1:
            print(f"Multiple devices found, using: {device}")
            print(f"Other devices: {', '.join(devices[1:])}")

    return monitor_serial(
        device,
        filter_keyword=args.filter,
        save_file=args.save,
        use_colors=not args.no_color,
        baud=args.baud
    )


if __name__ == '__main__':
    sys.exit(main())
