# CLAUDE.md

Guidance for Claude Code when building Precursor / Xous apps with this toolkit. This file is canonical: if another doc disagrees, this file and the xous-core source win.

Verified against xous-core at the commit in `XOUS_CORE_ANCHOR`. When in doubt, read the source in `$XOUS_ROOT` (`services/gam`, `services/pddb`, `apps/hello`, `apps/ball`, `apps/vault`), not memory.

## Definition of done

An app is not done until all of these are true. Don't mark anything shipped on assertion alone.

1. `bin/integrate --grade <repo>` passes at the anchor commit: hosted build with 0 errors, clippy clean, host tests in `core/` pass, `bin/lint` clean.
2. Pure logic (parsers, game rules, encoders, math) lives in `core/` (`<pkg>-core`, no Xous deps) with unit tests, including a regression test for every bug fixed.
3. A Renode capture run (`scripts/renode_capture.py`) exits 0 and its screenshots show the app, not Shellchat.
4. The README's claims match what the code does (no "secure", "production-ready", "network" features that aren't there).

## Start here

```bash
bin/new-app <name> "<Display Name>"      # scaffold; builds on first try
bin/integrate --grade <repo>            # the 10/10 gate: build, clippy, host tests, lint
bin/lint <repo>                         # banned patterns only (fast, no cargo)
```

The full bar for an app is in `ecosystem/REMEDIATION.md` ("What 10/10 means"). How work flows (audit → build → verify/retro → upstream → measure) is in `ecosystem/FLYWHEEL.md`. Read `ecosystem/LESSONS.md` before starting app work: it lists the patterns and reference apps to copy, and the mistakes earlier rounds made.

Build new apps on `precursor-appkit` (`appkit/`). Its pure parts (`keys`, `layout`, fail-closed `entropy`) live in `precursor-appkit-core/`, which an app's `core/` crate depends on by git (`precursor-appkit-core = { git = "https://github.com/tbcolby/xous-dev-toolkit" }`, plus `features = ["testing"]` in dev-dependencies for deterministic/failing entropy sources). `bin/integrate` patches that git source to the in-tree copy so app, core and appkit share one crate; don't copy `entropy.rs` into apps. Only drop to raw GAM/PDDB APIs for things the kit doesn't cover, and follow the rules below when you do.

## Hard rules (each one has broken real apps)

**Names**
- Two names per app. The xous-names server name (`"_Mines_"`) is any unique string. The GAM app name **must** be `gam::APP_NAME_<KEY>`, where `<KEY>` is the app's `apps/manifest.json` key, which must equal the Cargo package name (uppercased, `-` → `_`). Anything else and `register_ux` returns `Ok(None)`.
- Don't name a package after a crate already in the xous-core dependency graph (`cipher`, `regex`, `json`, …). `cargo -p` becomes ambiguous. `bin/new-app` rejects known collisions.

**Keys** (`services/keyboard/src/mappings/qwerty.rs`, `services/gam/src/contexts.rs:741`)

| Key | Char | Notes |
|---|---|---|
| ← ↑ → ↓ | U+2190 U+2191 U+2192 U+2193 | Never Apple PUA U+F700–F703 |
| Enter | `'\r'` (0x0D) | |
| Backspace | 0x08 | Use as Back on non-text screens |
| F1–F4 | 0x11–0x14 | Not U+F001… |
| `∴` menu / D-pad centre | U+2234 | **GAM consumes it. Apps never receive it.** Never bind Back/Quit/Menu to it |
| Escape | — | Doesn't exist on the device or in Renode |

Every screen needs an exit reachable with Backspace or F4.

**Lifecycle**
- Use `UxType::Framebuffer` + `rawkeys_id` for games and tools; `Chat` reserves an IME input line.
- Size from `gam.get_canvas_bounds(gid)`. The canvas is below the status bar; never assume 336×536.
- Handle `focuschange_id`: stop pumps/timers on `Background`, redraw on `Foreground`. Nothing should tick while another app is in front.
- Leave with `gam.relinquish_focus()`. Don't `terminate_process` on a user quit: apps start at boot and aren't respawned.
- `xous::destroy_server` is `unsafe`; wrap it with a safety comment.

**Drawing**
- `TextView::invert` is ignored for app canvases (trust level). Show selection with a border or marker.
- Call `gam.redraw()` once per frame, after drawing.

**Storage (PDDB)**
- There is no `PddbKey::set_len` or truncate. To replace a value: `delete_key` in the key's basis (from `attributes().basis`), recreate, write, `sync()`. `appkit::store::Store::save` does this.
- `basis = None`: reads see the union of open bases; *new* keys go to the most recently opened basis. Rewrite existing keys in their own basis.
- Reload cached data when the basis set changes (`appkit::store::watch_bases` / `pddb.monitor_basis()`). Never rewrite cached data unconditionally on focus loss: save only when dirty.
- Errors are `std::io::Error`. Treat `NotFound` as "no data yet"; treat a parse failure as "don't overwrite".
- Put a `version` field in stored JSON.

**Randomness**
- Fail closed. Never `get_u32().unwrap_or(0)`. Games: `appkit::rng::Trng` (panics on TRNG error) and its unbiased `below`/`shuffle`. Secrets: write core generators against `precursor_appkit_core::entropy::Entropy` (fallible; `Trng` implements it) so a TRNG failure becomes an error message, never a value.

**Build**
- Add apps to workspace `members` only, never `default-members` (`bin/integrate` does this).
- Crate versions match `apps/hello/Cargo.toml` at the anchor (xous 0.9.71, xous-ipc 0.10.11, xous-api-log 0.1.70, xous-api-names 0.9.72, xous-api-ticktimer 0.9.71).
- Path deps are `../../services/<svc>` / `../../libs/<lib>`. A repo-local library goes in `libs/<crate>/` in the app repo; `bin/integrate` copies it to `xous-core/libs/`.

## APIs that do not exist

Agents have repeatedly invented these. If you're about to type one, stop and read the source:

`graphics_server::*` in an app · `gam::RawkeysEvent` · `gam.clear()` · `gam.draw_textview()` · `Canvas` as an app type · `Ticktimer::set_alarm` · `xous::String::from_str` · `PddbKey::set_len` · `pddb::PddbError` · `net::Net` (it's `net::NetManager`)

## Specialist docs

Read `agents/<name>.md` when working in that area. `agents/README.md` has the full roster and routing.

| Area | File |
|---|---|
| Routing a task | `agents/supervisor.md` |
| App concept, UX | `agents/ideation.md` |
| State machines, opcodes, threads, IPC | `agents/architecture.md` |
| GAM, text, drawing, modals | `agents/graphics.md` |
| PDDB | `agents/storage.md` |
| TCP/HTTP/TLS | `agents/networking.md` |
| COM, battery, sensors, WiFi, power | `agents/system.md` |
| TRNG use | `agents/randomness.md` |
| QR / barcodes / encodings | `agents/encoding.md` |
| Cargo, manifest, xtask | `agents/build.md` |
| Renode, screenshots, tests | `agents/testing.md` |
| Review checklist | `agents/review.md` |

## Renode automation

`scripts/renode_lib.py` (`RenodeSession`, `RenodeController`) and `scripts/renode_capture.py`. Details and exit codes: `scripts/README.md`. Key facts:
- The keyboard has a 500 ms hold threshold. Use `timed_key()` for navigation, `inject_line()` for text and Enter.
- Renode scancodes: `Enter` (not `Return`), `BackSpace`, `ShiftL`/`ShiftR`, `Up`/`Down`/`Left`/`Right`, `F1`–`F4`, `Home` (= `∴`). No `Escape`.
- Per-app capture sequences live in the app repo as `capture.py` with `run(ctl, outdir) -> failures`.
- Default PDDB PIN for test images is `a`.

## Repository layout

```
bin/            new-app, integrate, lint, status
appkit/         precursor-appkit crate (Xous shell helpers)
precursor-appkit-core/  pure half: keys, layout, entropy (host-testable, git-dependable)
templates/app/  app template
scripts/        Renode + USB tooling (Python)
agents/         specialist docs
ecosystem/      STANDARDS, PIPELINE, TRACKER
tests/          pytest + doc-snippets compile check
XOUS_CORE_ANCHOR  verified xous-core commit
```
