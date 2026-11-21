"""Tests for bin/new-app, bin/integrate and bin/status (no cargo needed)."""
import importlib.machinery
import importlib.util
import json
import subprocess
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent


def load(name):
    loader = importlib.machinery.SourceFileLoader(name.replace("-", "_"), str(ROOT / "bin" / name))
    spec = importlib.util.spec_from_loader(loader.name, loader)
    mod = importlib.util.module_from_spec(spec)
    loader.exec_module(mod)
    return mod


integrate = load("integrate")
status = load("status")


def fake_xous(tmp: Path) -> Path:
    x = tmp / "xous-core"
    for d in ["xtask", "services/gam", "services/pddb", "services/trng", "libs/ux-api", "apps"]:
        (x / d).mkdir(parents=True)
    for d in ["services/gam", "services/pddb", "services/trng", "libs/ux-api"]:
        (x / d / "Cargo.toml").write_text('[package]\nname = "x"\n')
    (x / "Cargo.toml").write_text(
        'default-members = [\n  "services/gam",\n]\nmembers = [\n  "xous-rs",\n  "services/gam",\n]\n'
    )
    (x / "apps" / "manifest.json").write_text(json.dumps({"hello": {"context_name": "Hello", "menu_name": {}}}))
    subprocess.run(["git", "init", "-q", str(x)], check=True)
    return x


def run_new_app(tmp, *args):
    return subprocess.run([sys.executable, str(ROOT / "bin" / "new-app"), *args, "--dir", str(tmp)],
                          capture_output=True, text=True)


def test_new_app_substitutes_everything(tmp_path):
    r = run_new_app(tmp_path, "tally-demo", "Tally Demo")
    assert r.returncode == 0, r.stderr
    repo = tmp_path / "precursor-tally-demo"
    blob = "".join(p.read_text() for p in repo.rglob("*") if p.is_file() and ".git" not in p.parts)
    assert "{{" not in blob
    assert "gam::APP_NAME_TALLY_DEMO" in (repo / "src" / "main.rs").read_text()
    frag = json.loads((repo / "manifest-fragment.json").read_text())
    assert list(frag) == ["tally-demo"]
    assert frag["tally-demo"]["context_name"] == "Tally Demo"


@pytest.mark.parametrize("name", ["Bad", "1abc", "a", "has_underscore", "x" * 40])
def test_new_app_rejects_bad_names(tmp_path, name):
    assert run_new_app(tmp_path, name, "X").returncode != 0


def test_new_app_rejects_crate_collisions(tmp_path):
    r = run_new_app(tmp_path, "cipher", "Cipher")
    assert r.returncode != 0 and "collides" in r.stderr


def test_new_app_rejects_quote_in_display(tmp_path):
    assert run_new_app(tmp_path, "ok-name", 'Say "hi"').returncode != 0


def test_add_member_goes_to_members_not_default(tmp_path):
    x = fake_xous(tmp_path)
    integrate.add_member(x, "apps/foo")
    integrate.add_member(x, "apps/foo")  # idempotent
    text = (x / "Cargo.toml").read_text()
    default, members = text.split("members = [", 2)[1], text.split("\nmembers = [", 1)[1]
    assert '"apps/foo"' not in default.split("]")[0]
    assert members.count('"apps/foo"') == 1
    integrate.remove_member(x, "apps/foo")
    assert '"apps/foo"' not in (x / "Cargo.toml").read_text()


def make_app(tmp: Path, pkg: str, deps: str, frag=True, src='fn main() {}\n') -> Path:
    repo = tmp / f"precursor-{pkg}"
    (repo / "src").mkdir(parents=True)
    (repo / "Cargo.toml").write_text(f'[package]\nname = "{pkg}"\nversion = "0.1.0"\n\n[dependencies]\n{deps}')
    (repo / "src" / "main.rs").write_text(src)
    if frag:
        (repo / "manifest-fragment.json").write_text(json.dumps({pkg: {"context_name": pkg, "menu_name": {}}}))
    return repo


def test_integrate_good_app(tmp_path, monkeypatch):
    monkeypatch.setattr(integrate, "APPKIT", tmp_path / "appkit")
    (tmp_path / "appkit").mkdir()
    (tmp_path / "appkit" / "Cargo.toml").write_text('[package]\nname = "precursor-appkit"\n')
    x = fake_xous(tmp_path)
    repo = make_app(tmp_path, "good", 'gam = { path = "../../services/gam" }\n')
    warnings, broken = [], {}
    pkgs = integrate.integrate(x, [repo], warnings, broken)
    assert pkgs == ["good"] and not broken and not warnings
    assert '"apps/good"' in (x / "Cargo.toml").read_text()
    assert "good" in json.loads((x / "apps" / "manifest.json").read_text())


def test_integrate_quarantines_unresolved_paths(tmp_path, monkeypatch):
    monkeypatch.setattr(integrate, "APPKIT", tmp_path / "appkit")
    (tmp_path / "appkit").mkdir()
    (tmp_path / "appkit" / "Cargo.toml").write_text('[package]\nname = "precursor-appkit"\n')
    x = fake_xous(tmp_path)
    repo = make_app(tmp_path, "c64", 'gam = { path = "../../xous-core/services/gam" }\n')
    warnings, broken = [], {}
    integrate.integrate(x, [repo], warnings, broken)
    assert "c64" in broken
    assert '"apps/c64"' not in (x / "Cargo.toml").read_text()


def test_integrate_copies_repo_libs(tmp_path, monkeypatch):
    monkeypatch.setattr(integrate, "APPKIT", tmp_path / "appkit")
    (tmp_path / "appkit").mkdir()
    (tmp_path / "appkit" / "Cargo.toml").write_text('[package]\nname = "precursor-appkit"\n')
    x = fake_xous(tmp_path)
    repo = make_app(tmp_path, "othello", 'othello-core = { path = "../../libs/othello-core" }\n')
    (repo / "libs" / "othello-core").mkdir(parents=True)
    (repo / "libs" / "othello-core" / "Cargo.toml").write_text('[package]\nname = "othello-core"\n')
    warnings, broken = [], {}
    integrate.integrate(x, [repo], warnings, broken)
    assert not broken
    assert (x / "libs" / "othello-core" / "Cargo.toml").exists()


def test_integrate_flags_app_name_mismatch_and_missing_fragment(tmp_path, monkeypatch):
    monkeypatch.setattr(integrate, "APPKIT", tmp_path / "appkit")
    (tmp_path / "appkit").mkdir()
    (tmp_path / "appkit" / "Cargo.toml").write_text('[package]\nname = "precursor-appkit"\n')
    x = fake_xous(tmp_path)
    repo = make_app(tmp_path, "qrcode", "", frag=False, src="const N: &str = gam::APP_NAME_QRCODE_APP;\n")
    warnings, broken = [], {}
    integrate.integrate(x, [repo], warnings, broken)
    assert any("no manifest-fragment.json" in w for w in warnings)
    assert any("APP_NAME_QRCODE_APP" in w for w in warnings)


def test_integrate_refuses_upstream_app_names(tmp_path, monkeypatch):
    monkeypatch.setattr(integrate, "APPKIT", tmp_path / "appkit")
    (tmp_path / "appkit").mkdir()
    (tmp_path / "appkit" / "Cargo.toml").write_text('[package]\nname = "precursor-appkit"\n')
    x = fake_xous(tmp_path)
    (x / "apps" / "hello").mkdir()
    (x / "apps" / "hello" / "Cargo.toml").write_text('[package]\nname = "hello"\n')
    subprocess.run(["git", "-C", str(x), "add", "apps/hello"], check=True)
    repo = make_app(tmp_path, "hello", "")
    warnings, broken = [], {}
    assert integrate.integrate(x, [repo], warnings, broken) == []
    assert any("upstream" in w for w in warnings)


def test_status_block_replacement(tmp_path):
    data = {"xous_core": "abc1234", "anchor": "abc1234", "warnings": ["w1"],
            "results": {"a": {"ok": True, "errors": 0, "warnings": 0, "first_errors": []},
                        "b": {"ok": False, "errors": 2, "warnings": 1,
                              "first_errors": ["apps/b/src/main.rs:1:1: error[E0433]: failed | to resolve"]}}}
    j = tmp_path / "s.json"
    j.write_text(json.dumps(data))
    md = tmp_path / "T.md"
    md.write_text("# Tracker\n\nkeep me\n")
    sys.argv = ["status", str(j), "--update", str(md)]
    status.main()
    sys.argv = ["status", str(j), "--update", str(md)]
    status.main()  # second run replaces, not appends
    text = md.read_text()
    assert text.count(status.BEGIN) == 1 and "keep me" in text
    assert "**1/2 build.**" in text and "\\|" in text
