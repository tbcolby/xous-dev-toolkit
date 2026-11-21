# Xous Development Toolkit

Tools for building [Xous](https://github.com/betrusted-io/xous-core) apps for the [Precursor](https://www.crowdsupply.com/sutajio-kosagi/precursor) handheld, especially from macOS ARM64 where Renode has no GUI.

Everything here is verified against xous-core at the commit in [`XOUS_CORE_ANCHOR`](XOUS_CORE_ANCHOR). CI rebuilds a scaffolded app against that commit on every push, and against upstream `main` nightly.

## What's in the box

| Path | What it is |
|---|---|
| [`bin/new-app`](bin/new-app) | Scaffold a new app repo that builds, registers with GAM correctly, and uses `precursor-appkit` |
| [`bin/integrate`](bin/integrate) | Copy app repos into a xous-core tree (manifest, workspace, libs) and check each one. `--grade` = build + clippy + host tests + lint (the 10/10 gate) |
| [`bin/lint`](bin/lint) | Fast scan for patterns that broke real apps (PUA arrows, `∴` bindings, fail-open TRNG, `set_len`, `invert`, invented APIs, …) |
| [`bin/status`](bin/status) | Render integrate JSON as the generated status table in `ecosystem/TRACKER.md` |
| [`appkit/`](appkit/) | `precursor-appkit`: shared crate with correct key codes, GAM drawing helpers, PDDB JSON store, fail-closed TRNG, focus-aware pump |
| [`templates/app/`](templates/app/) | The app template `new-app` instantiates |
| [`scripts/`](scripts/README.md) | Renode automation (`renode_lib.py`, `renode_capture.py`) and the USB log monitor |
| [`agents/`](agents/README.md) | Specialist knowledge files for Claude Code (graphics, storage, networking, …) |
| [`ecosystem/`](ecosystem/) | App standards, pipeline, and portfolio tracker |
| [`tests/`](tests/) | Python tests; `tests/doc-snippets/` compiles the Rust snippets from the docs |
| [`.github/workflows/`](.github/workflows/) | Toolkit CI and a reusable `app-check.yml` for app repos |

## Quick start

```bash
# 0. One-time: a xous-core checkout at the anchor commit
git clone https://github.com/betrusted-io/xous-core ~/xous-core
git -C ~/xous-core checkout $(sed -n 's/^commit=//p' XOUS_CORE_ANCHOR)
export XOUS_ROOT=~/xous-core

# 1. New app
bin/new-app tally "Tally" --dir ~/Dev/precursor

# 2. Build it (hosted mode, no hardware needed)
bin/integrate --check ~/Dev/precursor/precursor-tally
cd $XOUS_ROOT && cargo xtask run tally          # opens the hosted-mode window

# 3. Whole portfolio
bin/integrate --check --json status.json ~/Dev/precursor/precursor-*
```

`bin/integrate` exits with the number of failing apps, so it works as a CI gate.

## precursor-appkit

The crate exists because the same mistakes kept recurring across apps. Each module is the fix for one of them:

| Module | Use it instead of… |
|---|---|
| `keys` | hand-written key constants. Arrows are U+2190–2193, Enter `\r`, Backspace 0x08, F1–F4 0x11–0x14. `∴` never reaches an app. |
| `rng` | `trng.get_u32().unwrap_or(0)` and `x % n`. Panics on TRNG failure; unbiased ranges and shuffles. |
| `layout` | ad-hoc list scrolling and hard-coded 336×536. |
| `store` | `PddbKey::set_len` (doesn't exist). Delete-then-rewrite in the key's own basis, `Result` everywhere, basis-change watcher. |
| `pump` | a ticker thread that runs while backgrounded or doubles its rate on refocus. |
| `screen` | `invert` (refused for apps), `gam.clear()`, `graphics_server::*`. |
| `app` | registering with the xous-names server name. Uses `gam::APP_NAME_*`, leaves via `relinquish_focus`. |

Host unit tests: `cargo test -p precursor-appkit --no-default-features` inside a xous-core tree that `bin/integrate` has populated.

## Hosted, Renode, hardware

| Target | Command | Use for |
|---|---|---|
| Hosted | `cargo xtask run <app>` | Fast iteration; window on your desktop |
| Renode | `cargo xtask renode-image <app>` then `scripts/renode_capture.py` | Screenshots, PDDB flows, CI-style capture |
| Device | `cargo xtask app-image <app>` + update via USB | The real thing |

## Setup (macOS ARM64)

1. **Renode**: download the ARM64 portable package from [Renode releases](https://github.com/renode/renode/releases) into `/Applications/`.
2. **Rust**: `rustup install stable`. For device and Renode images, install the Xous sysroot matching your rustc from [betrusted-io/rust releases](https://github.com/betrusted-io/rust/releases) into `~/.rustup/toolchains/stable-aarch64-apple-darwin/lib/rustlib/riscv32imac-unknown-xous-elf/`. Hosted builds need no sysroot.
3. **Flash backing file for Renode** (128 MiB of 0xFF):
   ```bash
   cd $XOUS_ROOT && python3 -c "open('tools/pddb-images/renode.bin','wb').write(b'\xff'*(128<<20))"
   ```
4. **Python**: 3.9+. `pip install pyserial` only for the USB log monitor. Dev: `pip install pytest ruff`.

## Renode capture

```bash
python3 scripts/renode_capture.py --init --app tally            # blank flash: format PDDB with PIN 'a'
python3 scripts/renode_capture.py --app tally --script ../precursor-tally/capture.py
```

Exit code is non-zero if any screenshot fails. See [`scripts/README.md`](scripts/README.md) for the plugin contract, exit codes and known limits.

## Using this with Claude Code

[`CLAUDE.md`](CLAUDE.md) is the canonical guide for agents. It is short on purpose: it lists the hard rules and points to `agents/<domain>.md` for detail. If guidance anywhere else disagrees with CLAUDE.md or with xous-core source, the source wins and the doc is a bug.

## License

Apache-2.0
