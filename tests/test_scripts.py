from __future__ import annotations

import renode_capture as rc
import usb_log_monitor as ulm


def test_line_splitter_handles_split_multibyte():
    s = ulm.LineSplitter()
    data = "café ↑\n".encode()
    out = []
    for i in range(len(data)):
        out += s.feed(data[i:i + 1])
    assert out == ["café ↑"]


def test_device_patterns_prefer_cu_on_macos():
    assert ulm.device_patterns("darwin") == ["/dev/cu.usbmodem*"]
    assert "/dev/ttyACM*" in ulm.device_patterns("linux")


def test_key_codes_no_fake_renode_fkeys():
    assert ulm.KEY_CODES[0x11] == "F1"
    assert 0xF001 not in ulm.KEY_CODES


def test_load_plugin_contract(tmp_path):
    p = tmp_path / "capture.py"
    p.write_text("def run(ctl, outdir):\n    return 2\n")
    run = rc.load_plugin(str(p))

    class Ctl:
        screenshot_failures = 0

    assert rc.run_capture(run, Ctl(), str(tmp_path)) == 2


def test_run_capture_counts_controller_failures(tmp_path):
    class Ctl:
        screenshot_failures = 3

    assert rc.run_capture(lambda c, d: 0, Ctl(), str(tmp_path)) == 3
    assert rc.run_capture(lambda c, d: None, Ctl(), str(tmp_path)) == 3


def test_load_plugin_without_run(tmp_path):
    p = tmp_path / "bad.py"
    p.write_text("x = 1\n")
    try:
        rc.load_plugin(str(p))
    except rc.ConfigError as e:
        assert "run(ctl, outdir)" in str(e)
    else:
        raise AssertionError("expected ConfigError")


def test_main_requires_xous_root(monkeypatch, tmp_path):
    monkeypatch.delenv("XOUS_ROOT", raising=False)
    assert rc.main(["--app", "calc"]) == rc.EXIT_CONFIG
    assert rc.main(["--xous-root", str(tmp_path / "missing"), "--app", "calc"]) == rc.EXIT_CONFIG


def test_main_unknown_app_in_build(tmp_path):
    d = tmp_path / "services" / "status" / "src"
    d.mkdir(parents=True)
    (d / "app_autogen.rs").write_text('fn app_menu_items() { t!("appmenu.calc", l) }\n')
    assert rc.main(["--xous-root", str(tmp_path), "--app", "nosuch",
                    "--screenshots", str(tmp_path / "o")]) == rc.EXIT_CONFIG
