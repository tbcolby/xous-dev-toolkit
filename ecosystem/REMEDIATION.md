# Portfolio remediation plan: every app to 10/10

Started 2026-11-20 after the portfolio audit (reports: `~/cognitive-infrastructure/platforms/precursor/docs/audits/2026-11-20/`). Baseline: 0/40 apps build at the anchor, mean audit score 3.5/10, 919 lint findings.

## What 10/10 means

An app scores 10/10 when it meets every gate below. G1–G4 are mechanical (`bin/integrate --grade`); R1–R4 are review gates checked against the audit report.

| Gate | Requirement | How it's checked |
|---|---|---|
| G1 | Hosted build at the anchor commit, 0 errors | `bin/integrate --grade` |
| G2 | Clippy clean (0 app warnings) | `--grade` (strict) |
| G3 | Pure logic in `core/` (`<pkg>-core`, no Xous deps), host tests pass, and every audit bug in pure logic has a regression test | `--grade` (host tests) + review |
| G4 | `bin/lint` clean; `manifest-fragment.json` key = package name | `--grade` (lint) |
| R1 | Every finding in the app's audit section is fixed, or explicitly marked won't-fix with a reason in the README | review vs report |
| R2 | Xous lifecycle: registers via `appkit::app::register` with `gam::APP_NAME_*`; right `UxType`; pump/timers stop on Background; every screen exits via Backspace/F4; leaves via `relinquish_focus`; saves only when dirty; reloads on basis change | review |
| R3 | README is truthful: features, keys, storage, limitations, no unverified security claims | review |
| R4 | `capture.py` Renode plugin and `.github/workflows/ci.yml` (app-check) present | file check |

Runtime verification on Renode/hardware is not possible in this environment. Apps reaching 10/10 here are verified at build, lint and host-test level; `capture.py` is written but unrun, and each README says so until a capture run passes.

## Target layout (every app)

```
precursor-<app>/
  Cargo.toml              bin crate: gam, precursor-appkit, <pkg>-core
  core/                   <pkg>-core: pure logic + unit tests, no xous/gam/pddb
  src/main.rs             message routing (template pattern)
  src/app.rs, src/ui.rs   state ↔ keys ↔ drawing via appkit::screen
  manifest-fragment.json  key == package name
  capture.py              Renode plugin: run(ctl, outdir) -> failures
  .github/workflows/ci.yml  uses xous-dev-toolkit app-check.yml
  README.md               features, keys, storage, limitations
```

Package renames needed for G4: `cipher` → `cipher-workshop` (crate collision; code already uses `APP_NAME_CIPHER_WORKSHOP`), `qrcode` → `qrcode-app` (code uses `APP_NAME_QRCODE_APP`). othello keeps `libs/othello-core` (integrate copies `libs/`).

## Strategy per app

**F** = fix in place (sound structure, migrate pieces to appkit). **R** = rebuild the shell (main/ui/storage) from `bin/new-app`, keeping and fixing the existing logic modules as `core/`.

| Round | Apps | Strategy | Key audit items (see report for full list) |
|---|---|---|---|
| 1a | barcode, qrcode, keygen, otp | R | Code 128 table + stop pattern, Code 39 rows; QR V7–10 version info; TRNG fail-closed; OTP pad reuse + full-pad view; deletes remove PDDB keys; differential tests vs reference encoders |
| 1b | cipher, contacts, budget, planner | R | basis handling (appkit store), id counters stored with data, compile errors beyond drift, dead keys, no secrets in Debug, zeroize where practical |
| 2a | calc, writer, timers | F | calc quit-on-unknown-key, F-key codes, RPN stack lift, unary minus/prefix fns; writer UTF-8 cursor, ESC prefix → F-keys (F1 menu, F2 preview, F4 back), truncating saves; timers background alerts (pump keeps running while an alert is armed), suspend-safe time |
| 2b | othello, c64, gameboy | F/F/R | othello move masks, AI pump, `target_os` gates, F-keys; c64 frame pump, PRG run, heap-allocate memory; gameboy rebuild UI, fix demos (halt, Fibonacci) |
| 3a | decide, mines, flashcards, carse | R/R/F/F | tournament propagation; mines borrow error + stale tails; flashcards index delete, TCP timeout + cancel + bind/auth; carse paused write loop, wall-clock drift |
| 3b | mathdrill, zork, fortress, pet | R | real GAM API, keys, persistence; pet real-time model + persistence |
| 4a | json, regex, rss, sqlite | R | parser robustness; regex backtracking bound + char classes; sqlite multi-column insert/create, no panics; rss panics; README truthful about no network |
| 4b | cron, gitlog, netscan, hwtools | R/R/R/F | cron next-fire; gitlog wire up parser; netscan honest scope; hwtools quit, settings save, no unrequested hardware writes |
| 5a | flames, penrose, kaleidoscope, sacred, moire | R | shared fixed-point trig (`appkit::fixed`), penrose scale + Robinson split, flames sampling, moire Zebra |
| 5b | lissajous, spirograph, reaction, life, death | R | overdraw, spirograph type error, Gray-Scott presets, stop when unfocused, death rule parser |

## Execution

- Two workers at a time (8-core / 16 GB machine), each with its own xous-core tree at the anchor so builds don't contend.
- Per app: clone to `~/Dev/precursor/<repo>`, work on `main`, commit in small steps, push after each app reaches the gate.
- Workers don't edit the toolkit. They list reusable helpers in their report; between rounds those are upstreamed into appkit, docs, lint and the template, then the next round uses them.
- After each round: `bin/integrate --grade --json ecosystem/status.json <all repos>` and `bin/status ecosystem/status.json --update ecosystem/TRACKER.md`, then commit.

## Status and TODO (paused 2026-11-21, usage limit)

**At 10/10 gate, pushed to `main` (11):** barcode, qrcode-app, keygen, otp, cipher-workshop, contacts, budget, planner (gate re-run by orchestrator) · calc, writer, timers (worker-reported gate pass + README review; orchestrator re-grade was interrupted).

**Resume checklist**
1. [ ] Re-grade calc, writer, timers independently: `bin/integrate --xous-root <tree> --grade ~/Dev/precursor/precursor-{calc,writer,timers}`.
2. [ ] Round 2b — resume from branch `remediation-wip` (pushed, NOT gated):
   - [ ] othello (F): WIP on `remediation-wip`; perft 1–6 = 4/12/56/244/1396/8200, AI off the UI thread, F-keys.
   - [ ] c64 (F): WIP on `remediation-wip` (was mid-way through vic.rs); path deps, frame pump, heap memory, 6502 tests.
   - [ ] gameboy (R): not started.
3. [ ] Round 3a: carse WIP on `remediation-wip` (core/ codec + draw started); decide, mines, flashcards not started.
4. [ ] Rounds 3b, 4a, 4b, 5a, 5b (table above) — 22 apps not started.
5. [ ] After each round: fold worker Retro into `LESSONS.md` + toolkit; `bin/integrate --grade --json ecosystem/status.json <all>`; `bin/status ecosystem/status.json --update ecosystem/TRACKER.md`; commit.
6. [ ] Renode capture runs for every app (needs Renode installed); until then READMEs say "Renode capture not yet run".
7. [ ] Final pass, every repo: README rebuilt around real Renode screenshots from `capture.py` (hero shot + one per screen, key table, honest limitations). Screenshots must come from a passing capture run (exit 0, app visible, not Shellchat); remove the "not yet run" notes only then.
8. [ ] Remaining appkit candidates (LESSONS "Open"): modal confirm/menu overlay, right-aligned text, non-blocking alert thread, device-clock → `Date` helper.

**How to run workers** (see `LESSONS.md`): model Sonnet; brief = REMEDIATION + LESSONS + CLAUDE.md + reference apps; one private xous-core tree and `CARGO_HOME` per worker; max two concurrent builds; orchestrator re-runs `--grade` and reviews against the audit before counting an app done; toolkit edits only in a `git worktree`.

## Log

- 2026-11-20/21: toolkit gate + appkit-core + helpers; 11 apps to 10/10; see `LESSONS.md` log for per-round notes.
