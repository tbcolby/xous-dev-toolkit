"""Test helpers: a fake Renode monitor and a tiny stdlib PNG encoder."""

from __future__ import annotations

import base64
import os
import socket
import struct
import sys
import threading
import zlib

import pytest

sys.path.insert(0, os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "scripts"))


def make_png(rows, width=None, filters=(0, 1, 2, 3, 4)):
    """Encode 8-bit grayscale rows (lists of ints) as PNG, cycling filter types."""
    width = width or len(rows[0])
    height = len(rows)

    def chunk(t, body):
        return struct.pack(">I", len(body)) + t + body + struct.pack(">I", zlib.crc32(t + body) & 0xFFFFFFFF)

    def paeth(a, b, c):
        p = a + b - c
        pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
        return a if pa <= pb and pa <= pc else (b if pb <= pc else c)

    raw = bytearray()
    prev = [0] * width
    for y, row in enumerate(rows):
        f = filters[y % len(filters)]
        raw.append(f)
        for x, v in enumerate(row):
            left = row[x - 1] if x else 0
            up = prev[x]
            ul = prev[x - 1] if x else 0
            pred = {0: 0, 1: left, 2: up, 3: (left + up) >> 1, 4: paeth(left, up, ul)}[f]
            raw.append((v - pred) & 0xFF)
        prev = row
    return (b"\x89PNG\r\n\x1a\n"
            + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 0, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(bytes(raw)))
            + chunk(b"IEND", b""))


def screen(width=16, height=48, status=0, content=0):
    """Rows: status bar region (0..32) filled with `status`, rest with `content` pattern."""
    rows = []
    for y in range(height):
        if y < 33:
            rows.append([(status + x) & 0xFF for x in range(width)])
        else:
            rows.append([(content * 7 + x * 3 + y) & 0xFF for x in range(width)])
    return make_png(rows)


def inline(png: bytes) -> bytes:
    """Wrap PNG bytes the way Renode prints a screenshot (iTerm2 inline image)."""
    b64 = base64.b64encode(png)
    # split across lines like a terminal would
    wrapped = b"\r\n".join(b64[i:i + 76] for i in range(0, len(b64), 76))
    return b"\x1b]1337;File=name=screenshot.png;inline=1:" + wrapped + b"\x07\r\n(machine-0) "


class FakeMonitor:
    """Single-connection TCP server that records commands.

    `responses` maps a command prefix to a list of byte payloads (or callables
    returning bytes) consumed in order; the last entry repeats.
    `on_connect` bytes are sent right after accept (stale output).
    """

    def __init__(self, responses=None, on_connect=b"Renode monitor\r\n(monitor) "):
        self.responses = responses or {}
        self.on_connect = on_connect
        self.commands = []
        self.srv = socket.socket()
        self.srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        self.srv.bind(("127.0.0.1", 0))
        self.srv.listen(1)
        self.port = self.srv.getsockname()[1]
        self._stop = False
        self.thread = threading.Thread(target=self._serve, daemon=True)
        self.thread.start()

    def _respond(self, cmd):
        for prefix, queue in self.responses.items():
            if cmd.startswith(prefix):
                item = queue.pop(0) if len(queue) > 1 else queue[0]
                return item() if callable(item) else item
        return b""

    def _serve(self):
        try:
            conn, _ = self.srv.accept()
        except OSError:
            return
        with conn:
            if self.on_connect:
                conn.sendall(self.on_connect)
            buf = b""
            while not self._stop:
                try:
                    data = conn.recv(4096)
                except OSError:
                    return
                if not data:
                    return
                buf += data
                while b"\n" in buf:
                    line, buf = buf.split(b"\n", 1)
                    cmd = line.decode("latin-1")
                    self.commands.append(cmd)
                    out = self._respond(cmd)
                    if out:
                        conn.sendall(out)

    def close(self):
        self._stop = True
        self.srv.close()


@pytest.fixture
def monitor_factory():
    made = []

    def make(**kw):
        m = FakeMonitor(**kw)
        made.append(m)
        return m

    yield make
    for m in made:
        m.close()


@pytest.fixture
def controller_for():
    from renode_lib import RenodeController

    made = []

    def make(mon, **kw):
        kw.setdefault("screenshot_timeout", 2.0)
        kw.setdefault("idle_timeout", 0.2)
        ctl = RenodeController(mon.port, sleep=lambda s: None, **kw)
        ctl.connect(retries=1)
        made.append(ctl)
        return ctl

    yield make
    for c in made:
        c.close()
