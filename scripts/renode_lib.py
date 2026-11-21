#!/usr/bin/env python3
"""
Renode automation library for Precursor / Xous development.

Drives a Renode emulator through its telnet monitor port: keyboard input,
text injection, screenshots, change detection, PDDB setup and app launch.

Typical use (the session owns the Renode process and always terminates it):

    from renode_lib import RenodeSession

    with RenodeSession(xous_root="/path/to/xous-core") as session:
        ctl = session.ctl
        ctl.timed_key("Home", after=3.0)
        ctl.screenshot("/tmp/screen.png")      # raises ScreenshotError on failure

Library code never calls sys.exit(); every failure is an exception derived
from RenodeError so callers can clean up and decide on an exit code.

Environment variables:
    XOUS_ROOT     xous-core checkout (required unless passed explicitly)
    RENODE_PATH   Renode binary (default: /Applications/Renode.app/Contents/MacOS/renode)
    RENODE_PORT   Monitor telnet port (default: 4567)

Facts this module relies on (checked against xous-core 31fbe23):
    * Scancodes: emulation/peripherals/keyboard.cs MatrixMapping (lines 278-339).
      Keys missing from that table are silently dropped (lines 182-186).
    * Inject path: keyboard.cs InjectKey/InjectString/InjectLine enqueue chars;
      services/keyboard/src/main.rs:103-109 forwards only the low 8 bits
      (char_reg & 0xff), so injected characters must be <= 0xFF.
    * Status bar: services/gam/src/main.rs:74-89 creates the status canvas as
      Rectangle(0, 0, width, 2 * glyph_height(Cjk)) with Cjk = 16 px
      (libs/blitstr2/src/glyphstyle.rs:60), i.e. rows 0..=32. App content starts
      at row 33 (services/gam/src/layouts/framebuffer.rs:30).
"""

from __future__ import annotations

import base64
import binascii
import hashlib
import logging
import os
import re
import socket
import struct
import subprocess
import tempfile
import time
import zlib
from typing import Callable, Iterable, List, Sequence, Tuple

log = logging.getLogger("renode")

DEFAULT_PORT = int(os.environ.get("RENODE_PORT", "4567"))
DEFAULT_RENODE_PATH = "/Applications/Renode.app/Contents/MacOS/renode"
DEFAULT_RESC = "emulation/xous-release.resc"
DEFAULT_PIN = "a"
DEFAULT_HOST = "127.0.0.1"

# Rows 0..32 belong to the status bar (see module docstring).
STATUS_BAR_HEIGHT = 33

# Xous keyboard service hold threshold is 500 ms; holds at or above it produce
# the alternate character or get dropped.
MAX_HOLD_MS = 499

PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"
PNG_IEND = b"IEND\xaeB`\x82"

# Every scancode the Renode keyboard peripheral actually maps
# (xous-core/emulation/peripherals/keyboard.cs:278-339). Note: Enter, not
# Return; BackSpace; ShiftL/ShiftR; there is no Escape, Tab, Ctrl or Alt.
SCANCODES = frozenset(
    [f"Number{i}" for i in range(10)]
    + list("QWERTYUIOPASDFGHJKLZXCVBNM")
    + [
        "BackSpace", "Tilde", "OemQuestion", "Enter",
        "ShiftL", "OemComma", "Space", "OemPeriod", "ShiftR",
        "F1", "F2", "F3", "F4",
        "Up", "Down", "Left", "Right",
        "KeypadEnter", "OemPipe", "Home",
    ]
)

# ANSI cursor sequences decoded by the keyboard service's inject path
# (services/keyboard/src/main.rs:906-981, esc_match at 1142-1180):
# ESC [ A/B/C/D -> U+2191 / U+2193 / U+2192 / U+2190, ESC [ 1 ~ -> U+2234 (menu).
ARROW_SEQUENCES = {
    "up": (0x1B, 0x5B, 0x41),
    "down": (0x1B, 0x5B, 0x42),
    "right": (0x1B, 0x5B, 0x43),
    "left": (0x1B, 0x5B, 0x44),
}
MENU_SEQUENCE = (0x1B, 0x5B, 0x31, 0x7E)


# --------------------------------------------------------------------------
# Exceptions
# --------------------------------------------------------------------------

class RenodeError(Exception):
    """Base class for every error raised by this module."""


class ConfigError(RenodeError):
    """Bad configuration: missing XOUS_ROOT, Renode binary, bad argument."""


class PortInUseError(RenodeError):
    """The monitor port is already open (probably a stale Renode)."""


class RenodeStartError(RenodeError):
    """Renode exited or never opened its monitor port."""


class MonitorConnectionError(RenodeError):
    """Could not connect to, or lost, the monitor socket."""


class ScreenshotError(RenodeError):
    """No valid PNG could be read from the monitor."""


class PngError(ValueError):
    """PNG could not be decoded by the minimal stdlib decoder."""


# --------------------------------------------------------------------------
# Pure helpers (unit tested)
# --------------------------------------------------------------------------

def resolve_xous_root(xous_root: str | None = None) -> str:
    """Return an absolute, existing xous-core path from the argument or $XOUS_ROOT."""
    root = xous_root or os.environ.get("XOUS_ROOT")
    if not root:
        raise ConfigError(
            "xous-core location unknown: pass --xous-root or set XOUS_ROOT"
        )
    root = os.path.abspath(os.path.expanduser(root))
    if not os.path.isdir(root):
        raise ConfigError(f"XOUS_ROOT does not exist or is not a directory: {root}")
    return root


def validate_scancode(key: str) -> str:
    if key not in SCANCODES:
        hint = ""
        if key == "Return":
            hint = " (use 'Enter', or inject_line('') for a CR)"
        elif key in ("Escape", "Esc"):
            hint = " (the Precursor keyboard has no ESC key)"
        elif key in ("Shift", "LeftShift", "RightShift"):
            hint = " (use 'ShiftL' or 'ShiftR')"
        elif key in ("Backspace", "Back"):
            hint = " (use 'BackSpace')"
        raise ValueError(f"unmapped Renode scancode {key!r}{hint}")
    return key


def validate_hold_ms(hold_ms: int) -> int:
    if isinstance(hold_ms, bool) or not isinstance(hold_ms, int):
        raise ValueError(f"hold_ms must be an int, got {hold_ms!r}")
    if not 1 <= hold_ms <= MAX_HOLD_MS:
        raise ValueError(
            f"hold_ms must be 1..{MAX_HOLD_MS} (Xous treats >=500 ms as a long press), got {hold_ms}"
        )
    return hold_ms


def format_time_interval(ms: int) -> str:
    """Format milliseconds as a Renode time interval 'H:M:S.mmm' (1000 -> '0:0:1.000')."""
    if isinstance(ms, bool) or not isinstance(ms, int) or ms < 0:
        raise ValueError(f"duration must be a non-negative int of ms, got {ms!r}")
    seconds, millis = divmod(ms, 1000)
    minutes, seconds = divmod(seconds, 60)
    hours, minutes = divmod(minutes, 60)
    return f"{hours}:{minutes}:{seconds}.{millis:03d}"


def escape_monitor_string(text: str) -> str:
    """Escape text for a double-quoted Renode monitor argument.

    Backslash and double quote are escaped. CR/LF are rejected because they
    would terminate the monitor command; use inject_key(13) / inject_line().
    """
    if not isinstance(text, str):
        raise TypeError("text must be str")
    if "\n" in text or "\r" in text:
        raise ValueError("text must not contain CR/LF; use inject_line() or inject_key(13)")
    bad = [c for c in text if ord(c) > 0xFF]
    if bad:
        raise ValueError(
            f"characters above U+00FF cannot be injected (keyboard service keeps the low byte): {bad!r}"
        )
    return text.replace("\\", "\\\\").replace('"', '\\"')


_B64_RUN = re.compile(rb"[A-Za-z0-9+/=\s]*")


def extract_png(data: bytes) -> bytes:
    """Return the PNG from the LAST complete image in monitor output.

    Renode prints screenshots using the iTerm2 inline-image escape
    (ESC ] 1337 ; File=...;inline=1:<base64> BEL). Earlier images in the
    buffer are stale and ignored. Raises ScreenshotError if the last image
    is missing, truncated, or not a PNG.
    """
    idx = data.rfind(b"inline=1:")
    if idx < 0:
        raise ScreenshotError("no inline image in monitor output")
    run = _B64_RUN.match(data, idx + len(b"inline=1:")).group(0)
    b64 = re.sub(rb"\s", b"", run)
    if not b64:
        raise ScreenshotError("inline image marker without data")
    try:
        png = base64.b64decode(b64, validate=True)
    except (binascii.Error, ValueError) as e:
        raise ScreenshotError(f"bad base64 in screenshot ({len(b64)} chars): {e}") from e
    if png[:8] != PNG_SIGNATURE:
        raise ScreenshotError("screenshot data is not a PNG (bad signature)")
    if not png.endswith(PNG_IEND):
        raise ScreenshotError(f"truncated PNG ({len(png)} bytes, no IEND chunk)")
    return png


def _paeth(a: int, b: int, c: int) -> int:
    p = a + b - c
    pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
    if pa <= pb and pa <= pc:
        return a
    if pb <= pc:
        return b
    return c


_CHANNELS = {0: 1, 2: 3, 3: 1, 4: 2, 6: 4}


def decode_png_rows(png: bytes) -> Tuple[int, int, List[bytes]]:
    """Decode a non-interlaced PNG into unfiltered scanlines (stdlib only).

    Returns (width, height, rows); each row is raw packed pixel bytes in the
    image's own format. Good enough to hash or compare regions; no palette or
    bit-depth expansion is done.
    """
    if png[:8] != PNG_SIGNATURE:
        raise PngError("not a PNG")
    pos = 8
    ihdr = None
    idat = bytearray()
    while pos + 8 <= len(png):
        (length,) = struct.unpack(">I", png[pos:pos + 4])
        ctype = png[pos + 4:pos + 8]
        body = png[pos + 8:pos + 8 + length]
        pos += 12 + length
        if ctype == b"IHDR":
            ihdr = struct.unpack(">IIBBBBB", body)
        elif ctype == b"IDAT":
            idat += body
        elif ctype == b"IEND":
            break
    if ihdr is None:
        raise PngError("missing IHDR")
    width, height, depth, color, _comp, _filt, interlace = ihdr
    if interlace != 0:
        raise PngError("interlaced PNG not supported")
    if color not in _CHANNELS:
        raise PngError(f"unknown color type {color}")
    bits_pp = depth * _CHANNELS[color]
    bpp = max(1, bits_pp // 8)
    stride = (width * bits_pp + 7) // 8
    try:
        raw = zlib.decompress(bytes(idat))
    except zlib.error as e:
        raise PngError(f"bad IDAT: {e}") from e
    if len(raw) < height * (stride + 1):
        raise PngError("IDAT shorter than image")
    rows: List[bytes] = []
    prev = bytearray(stride)
    for y in range(height):
        off = y * (stride + 1)
        ftype = raw[off]
        line = bytearray(raw[off + 1:off + 1 + stride])
        if ftype == 1:
            for i in range(bpp, stride):
                line[i] = (line[i] + line[i - bpp]) & 0xFF
        elif ftype == 2:
            for i in range(stride):
                line[i] = (line[i] + prev[i]) & 0xFF
        elif ftype == 3:
            for i in range(stride):
                left = line[i - bpp] if i >= bpp else 0
                line[i] = (line[i] + ((left + prev[i]) >> 1)) & 0xFF
        elif ftype == 4:
            for i in range(stride):
                left = line[i - bpp] if i >= bpp else 0
                upleft = prev[i - bpp] if i >= bpp else 0
                line[i] = (line[i] + _paeth(left, prev[i], upleft)) & 0xFF
        elif ftype != 0:
            raise PngError(f"bad filter type {ftype} on row {y}")
        rows.append(bytes(line))
        prev = line
    return width, height, rows


def content_hash(png: bytes, skip_top: int = STATUS_BAR_HEIGHT) -> str:
    """SHA-256 of the image below the status bar.

    The status bar shows uptime/battery/time and changes on its own, so it is
    excluded. Falls back to hashing the whole PNG (with a warning) if the
    image cannot be decoded.
    """
    try:
        width, height, rows = decode_png_rows(png)
    except PngError as e:
        log.warning("content_hash: cannot decode PNG (%s); hashing whole file", e)
        return hashlib.sha256(png).hexdigest()
    h = hashlib.sha256(struct.pack(">II", width, height))
    for row in rows[skip_top:]:
        h.update(row)
    return h.hexdigest()


def app_menu_index(xous_root: str, app_key: str) -> int:
    """Index of an app in the 'Switch to App...' submenu, derived from the build.

    The submenu is Shellchat (0), then every app in the order of
    services/status/src/app_autogen.rs app_menu_items() (regenerated by
    `cargo xtask` from apps/manifest.json for the apps in the image), then
    'Close menu'. app_key is the manifest key (e.g. 'calc', 'cipher-workshop').
    The answer is only right if the image in Renode was built from this tree.
    """
    path = os.path.join(xous_root, "services", "status", "src", "app_autogen.rs")
    try:
        with open(path, encoding="utf-8") as f:
            src = f.read()
    except OSError as e:
        raise ConfigError(f"cannot read {path} (build the image first): {e}") from e
    start = src.find("fn app_menu_items")
    if start < 0:
        raise ConfigError(f"app_menu_items() not found in {path}")
    keys = re.findall(r't!\("appmenu\.([^"]+)"', src[start:])
    if app_key not in keys:
        raise ConfigError(
            f"app {app_key!r} not in this build's app menu; available: {', '.join(keys) or '(none)'}"
        )
    return 1 + keys.index(app_key)


def port_in_use(port: int, host: str = DEFAULT_HOST, timeout: float = 0.5) -> bool:
    try:
        with socket.create_connection((host, port), timeout=timeout):
            return True
    except OSError:
        return False


# --------------------------------------------------------------------------
# Controller
# --------------------------------------------------------------------------

class RenodeController:
    """Talks to a running Renode monitor. Does not own the process.

    Use as a context manager to guarantee the socket is closed, or use
    RenodeSession to also own the Renode process.

    Attributes:
        screenshot_failures: count of screenshot() calls that raised.
    """

    def __init__(
        self,
        port: int | None = None,
        host: str = DEFAULT_HOST,
        *,
        io_timeout: float = 10.0,
        screenshot_timeout: float = 20.0,
        idle_timeout: float = 0.4,
        status_bar_height: int = STATUS_BAR_HEIGHT,
        sleep: Callable[[float], None] = time.sleep,
    ):
        self.port = port or DEFAULT_PORT
        self.host = host
        self.io_timeout = io_timeout
        self.screenshot_timeout = screenshot_timeout
        self.idle_timeout = idle_timeout
        self.status_bar_height = status_bar_height
        self.sleep = sleep
        self.sock: socket.socket | None = None
        self.screenshot_failures = 0
        self.last_png: bytes | None = None

    # -- connection -------------------------------------------------------

    def __enter__(self) -> RenodeController:
        if self.sock is None:
            self.connect()
        return self

    def __exit__(self, *exc) -> None:
        self.close()

    def connect(self, retries: int = 3, backoff: float = 2.0) -> None:
        last_error: BaseException | None = None
        for attempt in range(retries):
            try:
                self.sock = socket.create_connection((self.host, self.port), timeout=self.io_timeout)
                self.sleep(0.5)
                self._drain()
                self._send('mach set "SoC"')
                self.sleep(0.3)
                self._drain()
                return
            except OSError as e:
                last_error = e
                self.close()
                if attempt < retries - 1:
                    log.info("connection attempt %d/%d failed: %s", attempt + 1, retries, e)
                    self.sleep(backoff)
        raise MonitorConnectionError(
            f"cannot connect to Renode monitor {self.host}:{self.port} after {retries} attempts: {last_error}"
        )

    def close(self) -> None:
        if self.sock is not None:
            try:
                self.sock.close()
            except OSError:
                pass
        self.sock = None

    def quit(self) -> None:
        """Ask Renode to exit, then close the socket. Never raises."""
        if self.sock is not None:
            try:
                self._send("quit")
                self.sleep(1)
            except OSError:
                pass
        self.close()

    def _send(self, cmd: str) -> None:
        if self.sock is None:
            raise MonitorConnectionError("not connected")
        self.sock.sendall((cmd + "\n").encode("latin-1"))

    def command(self, cmd: str) -> None:
        """Send a raw monitor command (escape hatch for plugins)."""
        self._send(cmd)

    def _drain(self, quiet: float = 0.1, max_total: float = 2.0) -> int:
        """Discard pending monitor output; returns bytes discarded."""
        if self.sock is None:
            return 0
        total = 0
        deadline = time.monotonic() + max_total
        self.sock.settimeout(quiet)
        try:
            while time.monotonic() < deadline:
                try:
                    chunk = self.sock.recv(65536)
                except socket.timeout:
                    break
                if not chunk:
                    break
                total += len(chunk)
        finally:
            self.sock.settimeout(self.io_timeout)
        return total

    # -- keyboard -----------------------------------------------------------

    def timed_key(self, key: str, hold_ms: int = 1, after: float = 1.0) -> None:
        """Press a scancode for exactly hold_ms of *emulated* time.

        Pauses emulation, presses, RunFor hold_ms, releases, resumes. Needed
        because emulated time races ahead of wall clock while the CPU idles,
        so a plain Press/Release can exceed the 500 ms long-press threshold.
        Release and 'start' are always sent, even if the sequence fails.
        """
        self.timed_chord([key], hold_ms=hold_ms, after=after)

    def timed_chord(self, keys: Sequence[str], hold_ms: int = 1, after: float = 1.0) -> None:
        """Press several scancodes together (e.g. ['ShiftL', 'Number6'])."""
        keys = [validate_scancode(k) for k in keys]
        if not keys:
            raise ValueError("no keys given")
        validate_hold_ms(hold_ms)
        interval = format_time_interval(hold_ms)
        try:
            self._send("pause")
            self.sleep(0.2)
            for k in keys:
                self._send(f"sysbus.keyboard Press {k}")
                self.sleep(0.1)
            self._send(f'emulation RunFor "{interval}"')
            self.sleep(0.3)
        except BaseException:
            self._release_and_resume(keys, quiet=True)
            raise
        self._release_and_resume(keys, quiet=False)
        self.sleep(after)

    def _release_and_resume(self, keys: Sequence[str], quiet: bool) -> None:
        first_err: OSError | None = None
        for cmd in [f"sysbus.keyboard Release {k}" for k in reversed(keys)] + ["start"]:
            try:
                self._send(cmd)
                self.sleep(0.1)
            except (OSError, RenodeError) as e:
                first_err = first_err or e  # type: ignore[assignment]
        if first_err is not None and not quiet:
            raise first_err

    def inject_line(self, text: str = "", after: float = 0.5) -> None:
        """Inject text followed by CR. Bypasses key timing entirely.

        CR confirms PIN/text dialogs, fires the selected radio-dialog button,
        and dismisses "press any key" notifications.
        """
        self._send(f'sysbus.keyboard InjectLine "{escape_monitor_string(text)}"')
        self.sleep(after)

    def inject_string(self, text: str, after: float = 0.5) -> None:
        """Inject text without a trailing CR."""
        self._send(f'sysbus.keyboard InjectString "{escape_monitor_string(text)}"')
        self.sleep(after)

    def inject_key(self, char: str | int, after: float = 0.3) -> None:
        """Inject one character (str of length 1, or a code 1..255)."""
        code = ord(char) if isinstance(char, str) and len(char) == 1 else char
        if isinstance(code, bool) or not isinstance(code, int) or not 1 <= code <= 0xFF:
            raise ValueError(
                f"inject_key needs one character or a code 1..255 (keyboard service keeps the low byte), got {char!r}"
            )
        self._send(f"sysbus.keyboard InjectKey {code}")
        self.sleep(after)

    def _inject_codes(self, codes: Iterable[int], after: float) -> None:
        for c in codes:
            self._send(f"sysbus.keyboard InjectKey {c}")
        self.sleep(after)

    def inject_arrow(self, direction: str, after: float = 0.5) -> None:
        """Send an arrow key as the ANSI sequence ESC [ A/B/C/D via InjectKey.

        The keyboard service inject path accumulates ESC sequences and maps
        them to U+2191/2193/2192/2190 (xous-core services/keyboard/src/main.rs
        906-981, esc_match() at 1142-1180), and forwards them to the focused
        app only with the default 'debuginject' feature
        (services/keyboard/Cargo.toml:44).

        UNVERIFIED on a real Renode run: derived from source reading only.
        If it does nothing, fall back to timed_key('Up'/'Down'/'Left'/'Right').
        """
        seq = ARROW_SEQUENCES.get(direction.lower())
        if seq is None:
            raise ValueError(f"direction must be one of {sorted(ARROW_SEQUENCES)}, got {direction!r}")
        self._inject_codes(seq, after)

    def inject_menu_key(self, after: float = 0.5) -> None:
        """Send the menu key (U+2234) as ESC [ 1 ~ via InjectKey. UNVERIFIED, see inject_arrow."""
        self._inject_codes(MENU_SEQUENCE, after)

    # -- screen ---------------------------------------------------------------

    def _read_image_response(self) -> bytes:
        """Read monitor output until the last inline image is complete.

        Long timeout until the image marker appears, then stop on the BEL
        terminator or after idle_timeout with no new data.
        """
        assert self.sock is not None
        buf = bytearray()
        deadline = time.monotonic() + self.screenshot_timeout
        seen = False
        try:
            while True:
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    break
                self.sock.settimeout(min(remaining, self.idle_timeout if seen else 0.5))
                try:
                    chunk = self.sock.recv(65536)
                except socket.timeout:
                    if seen:
                        break
                    continue
                if not chunk:
                    raise MonitorConnectionError("monitor closed the connection")
                buf += chunk
                marker = buf.rfind(b"inline=1:")
                if marker >= 0:
                    seen = True
                    if buf.find(b"\x07", marker) >= 0:
                        break
        finally:
            self.sock.settimeout(self.io_timeout)
        return bytes(buf)

    def capture_png(self, retries: int = 2) -> bytes:
        """Take a screenshot and return validated PNG bytes. Raises ScreenshotError."""
        last: Exception | None = None
        for attempt in range(retries):
            self._drain()
            self._send("sysbus.memlcd TakeScreenshot")
            try:
                png = extract_png(self._read_image_response())
                self.last_png = png
                return png
            except ScreenshotError as e:
                last = e
                log.info("screenshot attempt %d/%d failed: %s", attempt + 1, retries, e)
        raise ScreenshotError(f"screenshot failed after {retries} attempts: {last}")

    def screenshot(self, filepath: str, retries: int = 2) -> int:
        """Save a screenshot to filepath; returns its size.

        Raises ScreenshotError (and increments screenshot_failures) on failure.
        """
        try:
            png = self.capture_png(retries)
        except ScreenshotError as e:
            self.screenshot_failures += 1
            log.error("  [%s] FAILED: %s", os.path.basename(filepath), e)
            raise
        os.makedirs(os.path.dirname(os.path.abspath(filepath)), exist_ok=True)
        with open(filepath, "wb") as f:
            f.write(png)
        log.info("  [%s] %d bytes", os.path.basename(filepath), len(png))
        return len(png)

    def try_screenshot(self, filepath: str, retries: int = 2) -> bool:
        """screenshot() that returns False instead of raising (failure still counted)."""
        try:
            self.screenshot(filepath, retries)
            return True
        except ScreenshotError:
            return False

    def screen_hash(self) -> str | None:
        """Hash of the content region (status bar excluded), or None on failure."""
        try:
            return content_hash(self.capture_png(), self.status_bar_height)
        except ScreenshotError as e:
            log.info("screen_hash: %s", e)
            return None

    def wait_for_screen_change(
        self, timeout: float = 60.0, interval: float = 2.0, reference: str | None = None
    ) -> bool:
        """Wait until the content region differs from `reference` (or from now).

        Status-bar changes (uptime clock) are ignored. Wall-clock timeout,
        measured with time.monotonic(), includes screenshot time.
        """
        deadline = time.monotonic() + timeout
        ref = reference if reference is not None else self.screen_hash()
        if ref is None:
            return False
        while time.monotonic() < deadline:
            self.sleep(interval)
            cur = self.screen_hash()
            if cur is not None and cur != ref:
                return True
        return False

    def wait_for_screen_stable(
        self, stable_for: float = 20.0, timeout: float = 600.0, interval: float = 5.0
    ) -> bool:
        """Wait until the content region is unchanged for `stable_for` seconds."""
        deadline = time.monotonic() + timeout
        last: str | None = None
        since = time.monotonic()
        while time.monotonic() < deadline:
            cur = self.screen_hash()
            now = time.monotonic()
            if cur is not None and cur == last:
                if now - since >= stable_for:
                    return True
            else:
                last, since = cur, now
            self.sleep(interval)
        return False

    # -- high-level sequences ---------------------------------------------------

    def confirm_radio_dialog(self) -> None:
        """Move to the [Okay] button of a two-option radio dialog and press CR."""
        self.timed_key("Down", after=1.5)
        self.timed_key("Down", after=1.5)
        self.inject_line("")

    def init_pddb(
        self,
        pin: str = DEFAULT_PIN,
        format_min_wait: float = 180.0,
        format_timeout: float = 900.0,
        stable_for: float = 60.0,
    ) -> None:
        """Format and unlock PDDB on blank flash.

        Instead of a fixed 15-minute sleep, waits at least format_min_wait and
        then until the screen has been unchanged for stable_for seconds (the
        unlock prompt), up to format_timeout. Raises RenodeError on timeout.
        The stable_for heuristic is UNVERIFIED against a real format run; if
        the progress dialog pauses longer than stable_for, raise it.
        """
        log.info("[PDDB] confirming format dialog")
        self.confirm_radio_dialog()
        self.sleep(10)
        log.info("[PDDB] setting PIN")
        self.inject_line(pin)
        self.sleep(10)
        log.info("[PDDB] dismissing notification")
        self.inject_line("")
        self.sleep(10)
        log.info("[PDDB] confirming PIN")
        self.inject_line(pin)
        self.sleep(5)

        log.info("[PDDB] formatting (min %.0fs, cap %.0fs)", format_min_wait, format_timeout)
        start = time.monotonic()
        self.sleep(format_min_wait)
        remaining = max(0.0, format_timeout - (time.monotonic() - start))
        if not self.wait_for_screen_stable(stable_for=stable_for, timeout=remaining):
            raise RenodeError(f"PDDB format did not settle within {format_timeout:.0f}s")
        log.info("[PDDB] format settled after %.0fs", time.monotonic() - start)
        self.unlock_pddb(pin)

    def unlock_pddb(self, pin: str = DEFAULT_PIN, mount_timeout: float = 120.0) -> bool:
        """Enter the PIN and wait for the screen to settle. Returns False on timeout."""
        log.info("[PDDB] unlocking")
        self.inject_line(pin)
        self.sleep(10)
        ok = self.wait_for_screen_stable(stable_for=10.0, timeout=mount_timeout, interval=3.0)
        if not ok:
            log.warning("[PDDB] screen did not settle within %.0fs after unlock", mount_timeout)
        return ok

    def launch_app(self, app_index: int, init_wait: float = 10.0) -> None:
        """Open main menu -> 'Switch to App...' -> item app_index.

        app_index: 0 = Shellchat, 1.. = apps in build order; prefer
        app_menu_index(xous_root, key) over a hand-found number.
        """
        if isinstance(app_index, bool) or not isinstance(app_index, int) or not 0 <= app_index <= 128:
            raise ValueError(f"app_index must be an int 0..128, got {app_index!r}")
        log.info("[NAV] opening main menu")
        self.timed_key("Home", after=3.0)
        self.timed_key("Down", after=1.0)  # skip "Sleep"
        self.timed_key("Home", after=3.0)  # "Switch to App..."
        log.info("[NAV] selecting app menu item %d", app_index)
        for _ in range(app_index):
            self.timed_key("Down", after=1.0)
        self.timed_key("Home", after=5.0)
        self.sleep(init_wait)


# --------------------------------------------------------------------------
# Process ownership
# --------------------------------------------------------------------------

def start_renode(
    xous_root: str | None = None,
    *,
    port: int | None = None,
    renode_path: str | None = None,
    resc: str = DEFAULT_RESC,
    log_path: str | None = None,
    startup_timeout: float = 30.0,
    host: str = DEFAULT_HOST,
) -> Tuple[subprocess.Popen, str]:
    """Start Renode and wait for its monitor port.

    Returns (process, log_path). Renode stdout/stderr go to log_path.
    Raises PortInUseError if the port is already open (a stale Renode would
    otherwise be driven instead), RenodeStartError if Renode exits or never
    opens the port, ConfigError for bad paths. The caller owns the process;
    prefer RenodeSession, which always terminates it.
    """
    root = resolve_xous_root(xous_root)
    port = port or DEFAULT_PORT
    renode_path = renode_path or os.environ.get("RENODE_PATH", DEFAULT_RENODE_PATH)
    if not os.path.exists(renode_path):
        raise ConfigError(f"Renode not found at {renode_path}; set RENODE_PATH")
    if '"' in root or '"' in resc:
        raise ConfigError("paths containing '\"' are not supported")
    if port_in_use(port, host):
        raise PortInUseError(
            f"monitor port {port} is already in use (stale Renode? try: lsof -i :{port}); refusing to start"
        )
    log_path = log_path or os.path.join(tempfile.gettempdir(), f"renode-{port}.log")
    logf = open(log_path, "ab")
    try:
        log.info("starting Renode (log: %s)", log_path)
        proc = subprocess.Popen(
            [renode_path, "--disable-xwt", "-P", str(port),
             "-e", f'path add @"{root}"; i @{resc}'],
            stdout=logf, stderr=subprocess.STDOUT, stdin=subprocess.DEVNULL, cwd=root,
        )
    finally:
        logf.close()  # child keeps its own descriptor
    deadline = time.monotonic() + startup_timeout
    while time.monotonic() < deadline:
        rc = proc.poll()
        if rc is not None:
            raise RenodeStartError(f"Renode exited with code {rc} during startup; see {log_path}")
        if port_in_use(port, host, timeout=1.0):
            log.info("monitor ready on port %d", port)
            return proc, log_path
        time.sleep(0.5)
    stop_process(proc)
    raise RenodeStartError(f"Renode monitor not ready after {startup_timeout:.0f}s; see {log_path}")


def stop_process(proc: subprocess.Popen | None, timeout: float = 5.0) -> None:
    """Terminate, then kill, a child process. Never raises."""
    if proc is None or proc.poll() is not None:
        return
    try:
        proc.terminate()
        proc.wait(timeout=timeout)
    except subprocess.TimeoutExpired:
        proc.kill()
        try:
            proc.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            log.error("Renode pid %d did not die", proc.pid)
    except OSError:
        pass


class RenodeSession:
    """Owns a Renode process plus a connected RenodeController.

        with RenodeSession(xous_root=...) as s:
            s.ctl.screenshot("x.png")

    On exit (normal or exception) it sends 'quit', closes the socket and
    terminates/kills the process.
    """

    def __init__(
        self,
        xous_root: str | None = None,
        *,
        port: int | None = None,
        renode_path: str | None = None,
        resc: str = DEFAULT_RESC,
        log_path: str | None = None,
        startup_timeout: float = 30.0,
        controller_kwargs: dict | None = None,
    ):
        self.xous_root = resolve_xous_root(xous_root)
        self.port = port or DEFAULT_PORT
        self.renode_path = renode_path
        self.resc = resc
        self.log_path = log_path
        self.startup_timeout = startup_timeout
        self.controller_kwargs = controller_kwargs or {}
        self.proc: subprocess.Popen | None = None
        self.ctl: RenodeController | None = None

    def start(self) -> RenodeSession:
        try:
            self.proc, self.log_path = start_renode(
                self.xous_root, port=self.port, renode_path=self.renode_path,
                resc=self.resc, log_path=self.log_path, startup_timeout=self.startup_timeout,
            )
            self.ctl = RenodeController(self.port, **self.controller_kwargs)
            self.ctl.connect()
            self.ctl.command("start")
            self.ctl.sleep(1)
        except BaseException:
            self.stop()
            raise
        return self

    def stop(self) -> None:
        if self.ctl is not None:
            self.ctl.quit()
        stop_process(self.proc)

    def __enter__(self) -> RenodeSession:
        return self.start()

    def __exit__(self, *exc) -> None:
        self.stop()


def reset_flash(xous_root: str | None = None) -> str:
    """Overwrite tools/pddb-images/renode.bin with a blank 128 MiB (0xFF) image."""
    root = resolve_xous_root(xous_root)
    path = os.path.join(root, "tools", "pddb-images", "renode.bin")
    os.makedirs(os.path.dirname(path), exist_ok=True)
    log.info("resetting flash to blank: %s", path)
    chunk = b"\xff" * (1024 * 1024)
    with open(path, "wb") as f:
        for _ in range(128):
            f.write(chunk)
    return path
