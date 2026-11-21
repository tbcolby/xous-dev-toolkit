from __future__ import annotations

import os
import socket
import stat
import sys
import time

import pytest

import renode_lib as rl
from conftest import inline, make_png, screen


def wait_for(pred, timeout=2.0):
    end = time.monotonic() + timeout
    while time.monotonic() < end:
        if pred():
            return True
        time.sleep(0.01)
    return pred()


# --- screenshot parsing ------------------------------------------------------

def test_extract_png_takes_last_match():
    old, new = screen(content=1), screen(content=2)
    assert rl.extract_png(inline(old) + b"junk" + inline(new)) == new


def test_extract_png_rejects_truncated_data():
    data = inline(screen())
    with pytest.raises(rl.ScreenshotError):
        rl.extract_png(data[: len(data) // 2])


def test_extract_png_rejects_non_png():
    import base64
    data = b"inline=1:" + base64.b64encode(b"GIF89a not a png at all") + b"\x07"
    with pytest.raises(rl.ScreenshotError, match="not a PNG"):
        rl.extract_png(data)


def test_extract_png_no_marker():
    with pytest.raises(rl.ScreenshotError):
        rl.extract_png(b"(machine-0) ")


def test_screenshot_drains_stale_frame_and_saves_fresh(monitor_factory, controller_for, tmp_path):
    stale, fresh = screen(content=1), screen(content=2)
    mon = monitor_factory(responses={"sysbus.memlcd TakeScreenshot": [inline(fresh)]})
    ctl = controller_for(mon)
    # A stale frame sits in the socket buffer (late reply to an earlier request).
    mon.responses["echo-stale"] = [inline(stale)]
    ctl.command("echo-stale")
    assert wait_for(lambda: "echo-stale" in mon.commands)
    time.sleep(0.05)
    out = tmp_path / "shot.png"
    n = ctl.screenshot(str(out))
    assert out.read_bytes() == fresh
    assert n == len(fresh)
    assert ctl.screenshot_failures == 0


def test_screenshot_reads_split_response(monitor_factory, controller_for, tmp_path):
    fresh = screen(content=3)
    payload = inline(fresh)
    mon = monitor_factory(responses={"sysbus.memlcd TakeScreenshot": [payload]})
    ctl = controller_for(mon)
    assert ctl.screenshot(str(tmp_path / "a.png")) == len(fresh)


def test_screenshot_partial_data_raises_and_counts(monitor_factory, controller_for, tmp_path):
    payload = inline(screen())
    partial = payload[: len(payload) // 2]  # no BEL, truncated base64
    mon = monitor_factory(responses={"sysbus.memlcd TakeScreenshot": [partial]})
    ctl = controller_for(mon)
    start = time.monotonic()
    with pytest.raises(rl.ScreenshotError):
        ctl.screenshot(str(tmp_path / "bad.png"))
    assert time.monotonic() - start < 3.0  # idle timeout, not a long tail
    assert ctl.screenshot_failures == 1
    assert not (tmp_path / "bad.png").exists()
    assert ctl.try_screenshot(str(tmp_path / "bad2.png")) is False
    assert ctl.screenshot_failures == 2


def test_screenshot_bad_png_raises(monitor_factory, controller_for, tmp_path):
    import base64
    bad = b"\x1b]1337;File=x;inline=1:" + base64.b64encode(b"hello world, definitely no png") + b"\x07"
    mon = monitor_factory(responses={"sysbus.memlcd TakeScreenshot": [bad]})
    ctl = controller_for(mon)
    with pytest.raises(rl.ScreenshotError):
        ctl.screenshot(str(tmp_path / "x.png"))


def test_screenshot_no_response_times_out(monitor_factory, controller_for, tmp_path):
    mon = monitor_factory()
    ctl = controller_for(mon, screenshot_timeout=0.5)
    with pytest.raises(rl.ScreenshotError):
        ctl.screenshot(str(tmp_path / "x.png"), retries=1)


# --- PNG decode / content region ------------------------------------------------

def test_decode_png_rows_roundtrip_all_filters():
    rows = [[(x * 13 + y * 7) & 0xFF for x in range(20)] for y in range(12)]
    w, h, out = rl.decode_png_rows(make_png(rows))
    assert (w, h) == (20, 12)
    assert [list(r) for r in out] == rows


def test_content_hash_ignores_status_bar():
    a = screen(status=1, content=5)
    b = screen(status=99, content=5)
    c = screen(status=1, content=6)
    assert a != b
    assert rl.content_hash(a) == rl.content_hash(b)
    assert rl.content_hash(a) != rl.content_hash(c)


def test_content_hash_falls_back_on_undecodable():
    assert len(rl.content_hash(rl.PNG_SIGNATURE + b"garbage")) == 64


def test_wait_for_screen_change_ignores_status_bar(monitor_factory, controller_for):
    frames = [inline(screen(status=i, content=0)) for i in range(50)]
    mon = monitor_factory(responses={"sysbus.memlcd TakeScreenshot": frames})
    ctl = controller_for(mon)
    assert ctl.wait_for_screen_change(timeout=0.8, interval=0.0) is False


def test_wait_for_screen_change_detects_content(monitor_factory, controller_for):
    frames = [inline(screen(status=0, content=0)), inline(screen(status=1, content=0)),
              inline(screen(status=2, content=9))]
    mon = monitor_factory(responses={"sysbus.memlcd TakeScreenshot": frames})
    ctl = controller_for(mon)
    assert ctl.wait_for_screen_change(timeout=3.0, interval=0.0) is True


def test_wait_for_screen_stable(monitor_factory, controller_for):
    frames = [inline(screen(content=1)), inline(screen(content=2)), inline(screen(status=5, content=3))]
    mon = monitor_factory(responses={"sysbus.memlcd TakeScreenshot": frames})
    ctl = controller_for(mon)
    assert ctl.wait_for_screen_stable(stable_for=0.3, timeout=3.0, interval=0.05) is True


# --- escaping / formatting / validation -------------------------------------------

def test_escape_monitor_string():
    assert rl.escape_monitor_string('a"b\\c') == 'a\\"b\\\\c'
    with pytest.raises(ValueError):
        rl.escape_monitor_string("line\n")
    with pytest.raises(ValueError):
        rl.escape_monitor_string("↑")


def test_inject_commands_are_escaped(monitor_factory, controller_for):
    mon = monitor_factory()
    ctl = controller_for(mon)
    ctl.inject_line('p"w\\d')
    ctl.inject_string('say "hi"')
    assert wait_for(lambda: len(mon.commands) >= 3)
    assert 'sysbus.keyboard InjectLine "p\\"w\\\\d"' in mon.commands
    assert 'sysbus.keyboard InjectString "say \\"hi\\""' in mon.commands


def test_format_time_interval():
    assert rl.format_time_interval(1) == "0:0:0.001"
    assert rl.format_time_interval(499) == "0:0:0.499"
    assert rl.format_time_interval(1000) == "0:0:1.000"
    assert rl.format_time_interval(3_723_045) == "1:2:3.045"
    with pytest.raises(ValueError):
        rl.format_time_interval(-1)


@pytest.mark.parametrize("bad", [0, 500, 1000, -3, 1.5, True])
def test_hold_ms_validation(bad):
    with pytest.raises(ValueError):
        rl.validate_hold_ms(bad)


def test_timed_key_sends_runfor(monitor_factory, controller_for):
    mon = monitor_factory()
    ctl = controller_for(mon)
    ctl.timed_key("Enter", hold_ms=250, after=0)
    assert wait_for(lambda: "start" in mon.commands[1:])
    seq = mon.commands[1:]
    assert seq == ["pause", "sysbus.keyboard Press Enter", 'emulation RunFor "0:0:0.250"',
                   "sysbus.keyboard Release Enter", "start"]


@pytest.mark.parametrize("key", ["Return", "Escape", "Shift", "Backspace", "Tab"])
def test_unmapped_scancodes_rejected(key):
    with pytest.raises(ValueError):
        rl.validate_scancode(key)


def test_scancodes_match_peripheral():
    for k in ("Enter", "BackSpace", "ShiftL", "ShiftR", "Home", "F4", "Number0", "OemPipe"):
        assert k in rl.SCANCODES
    assert "Escape" not in rl.SCANCODES and "Return" not in rl.SCANCODES


def test_timed_chord_releases_and_resumes_on_error():
    sent = []

    class Boom(rl.RenodeController):
        def _send(self, cmd):
            sent.append(cmd)
            if cmd.startswith("emulation RunFor"):
                raise OSError("socket died")

    ctl = Boom(1, sleep=lambda s: None)
    with pytest.raises(OSError):
        ctl.timed_chord(["ShiftL", "Number6"])
    assert sent[-3:] == ["sysbus.keyboard Release Number6", "sysbus.keyboard Release ShiftL", "start"]


def test_inject_key_and_arrow(monitor_factory, controller_for):
    mon = monitor_factory()
    ctl = controller_for(mon)
    ctl.inject_key("+")
    ctl.inject_arrow("down")
    assert wait_for(lambda: len(mon.commands) >= 5)
    assert mon.commands[1:5] == ["sysbus.keyboard InjectKey 43", "sysbus.keyboard InjectKey 27",
                                 "sysbus.keyboard InjectKey 91", "sysbus.keyboard InjectKey 66"]
    with pytest.raises(ValueError):
        ctl.inject_key("↑")
    with pytest.raises(ValueError):
        ctl.inject_arrow("sideways")


def test_launch_app_validates_index():
    ctl = rl.RenodeController(1, sleep=lambda s: None)
    for bad in (-1, 1000, "2", None):
        with pytest.raises(ValueError):
            ctl.launch_app(bad)


# --- config / process ---------------------------------------------------------------

def test_resolve_xous_root(tmp_path, monkeypatch):
    monkeypatch.delenv("XOUS_ROOT", raising=False)
    with pytest.raises(rl.ConfigError, match="XOUS_ROOT"):
        rl.resolve_xous_root(None)
    with pytest.raises(rl.ConfigError, match="does not exist"):
        rl.resolve_xous_root(str(tmp_path / "nope"))
    monkeypatch.setenv("XOUS_ROOT", str(tmp_path))
    assert rl.resolve_xous_root(None) == str(tmp_path)


def test_app_menu_index(tmp_path):
    d = tmp_path / "services" / "status" / "src"
    d.mkdir(parents=True)
    (d / "app_autogen.rs").write_text(
        'fn app_dispatch() { t!("appmenu.zzz", x) }\n'
        'pub(crate) fn app_menu_items(m: &mut Vec<MenuItem>, c: u32) {\n'
        '  name: String::from(t!("appmenu.barcode", locales::LANG)),\n'
        '  name: String::from(t!("appmenu.calc", locales::LANG)),\n}\n'
    )
    assert rl.app_menu_index(str(tmp_path), "barcode") == 1
    assert rl.app_menu_index(str(tmp_path), "calc") == 2
    with pytest.raises(rl.ConfigError, match="available: barcode, calc"):
        rl.app_menu_index(str(tmp_path), "zzz")


def test_start_refuses_when_port_in_use(tmp_path):
    srv = socket.socket()
    srv.bind(("127.0.0.1", 0))
    srv.listen(1)
    port = srv.getsockname()[1]
    marker = tmp_path / "ran"
    fake = tmp_path / "renode"
    fake.write_text(f"#!/bin/sh\ntouch {marker}\n")
    fake.chmod(fake.stat().st_mode | stat.S_IEXEC)
    try:
        with pytest.raises(rl.PortInUseError):
            rl.start_renode(str(tmp_path), port=port, renode_path=str(fake))
        assert not marker.exists()  # never launched
    finally:
        srv.close()


@pytest.mark.skipif(sys.platform == "win32", reason="posix shell script")
def test_start_detects_early_exit(tmp_path):
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        port = s.getsockname()[1]
    fake = tmp_path / "renode"
    fake.write_text("#!/bin/sh\necho boom\nexit 3\n")
    fake.chmod(fake.stat().st_mode | stat.S_IEXEC)
    logf = tmp_path / "renode.log"
    start = time.monotonic()
    with pytest.raises(rl.RenodeStartError, match="code 3"):
        rl.start_renode(str(tmp_path), port=port, renode_path=str(fake), log_path=str(logf))
    assert time.monotonic() - start < 10
    assert "boom" in logf.read_text()


def test_start_missing_renode(tmp_path):
    with pytest.raises(rl.ConfigError, match="Renode not found"):
        rl.start_renode(str(tmp_path), port=1, renode_path=str(tmp_path / "missing"))


def test_reset_flash(tmp_path):
    path = rl.reset_flash(str(tmp_path))
    assert os.path.getsize(path) == 128 * 1024 * 1024
