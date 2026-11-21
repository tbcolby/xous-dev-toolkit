# Lessons from app work

Every worker reads this before starting. After each work period the orchestrator adds what went wrong or took too long, and moves each fix into the toolkit (appkit, lint, template, CLAUDE.md). An entry here without a toolkit change means "not automated yet".

Format: **Lesson** · what happened · what changed in the toolkit.

## Process

1. **Read the reference apps before writing code.** Round 1 workers that started from the template and a 10/10 sibling (keygen → otp) finished faster with fewer gate iterations than ones that started from the old source. · Toolkit: the worker brief lists reference apps per pattern (below).
2. **Verify a claim about the toolkit before reporting it.** A worker reported `RenodeController.sleep` missing; it is an attribute set in `__init__`. · Toolkit: none needed; check with `grep -n` before reporting.
3. **Never edit the toolkit working tree that workers build from.** An in-progress appkit refactor on `main`'s checkout broke `bin/integrate` for a running worker. · Orchestrator rule: toolkit changes are made in a `git worktree`, tested in a spare xous-core tree, then fast-forwarded.
4. **Grade with the pinned toolchain.** CI on Rust 1.100 found clippy warnings that local 1.88 didn't. · Toolkit: `rust=` in `XOUS_CORE_ANCHOR`; CI installs it; `integrate` warns on mismatch.
6. **Use a private `CARGO_HOME` when `~/.cargo` is locked.** Other sessions on this machine hold the package-cache lock for long stretches; a cargo command then hangs with no output. Seed one per worker with an APFS clone (cheap): `cp -cR ~/.cargo/registry <scratch>/cargo-home-wN/registry && export CARGO_HOME=<scratch>/cargo-home-wN`. A crate with no dependencies needs only an empty `CARGO_HOME` plus `--offline`.
7. **Every commit should build.** A round-2a commit removed the old shell before the new one landed, so bisecting would hit a broken tree. Order commits core → shell → docs, each compiling, or squash.
8. **Don't overload the machine.** Two worker builds plus the orchestrator's verification pushed load past 40 on 8 cores (other sessions' jobs too). Orchestrator verification runs one build at a time.
10. **Tests must model the device, not the test harness.** Budget/planner created a fresh seeded test RNG per keypress, so every save drew the same "random" key and silently overwrote the previous record — and the tests passed. · Toolkit: `records::fresh_key` and `Store::create_record` refuse keys already in use; test with one long-lived entropy source and assert distinct keys.
11. **In an R rebuild, delete the old shell files.** Unused `src/ui.rs`/`src/cpu.rs` from the old app were still scanned by lint and confused reviewers. `git rm` anything the new shell doesn't `mod`.
12. **`bin/integrate -v` shows lint findings** (added round 2b); you don't need a separate `bin/lint` run to see why the gate failed.
13. **Clippy in `core/` first.** Most round-3a gate failures were `uninlined_format_args` in core crates; `cd core && cargo clippy --all-targets -- -D warnings` catches them in seconds, a `--grade` cycle takes 4–5 minutes.
14. **`git reset` before building commits.** Twice a staged `git rm` of the old shell leaked into the "core" commit, so that commit didn't build. Stage each commit explicitly (`git add core/` etc.) from a clean index.
15. **`precursor_appkit` re-exports `text`, `records`, `date`** (fixed after round 3a; it used to re-export only entropy/keys/layout and `use precursor_appkit::text` failed with E0432).
16. **Carry remainders when converting time units.** pet truncated each `elapsed` call to whole seconds and reset the baseline, losing up to 1 s per sync (~3% slow at a 30 s pump with the wall clock unset, worse with key-triggered syncs). Its tests passed because none called it more often than once a second. · Toolkit: shared `clock::Timekeeper` with a carry and a frequent-call regression test.
9. **Run the gate early, not just at the end.** Gate failures found at the end cost a full rebuild cycle each. Run `bin/lint` and `core` tests continuously; run `--grade` after the shell compiles.

## Patterns (use these, don't reinvent)

| Need | Use | Reference |
|---|---|---|
| Randomness for secrets | `precursor_appkit_core::entropy` (fallible; `Trng` implements `Entropy`) | keygen `core/src/gen.rs` |
| Randomness for games | `appkit::rng::Trng::below/shuffle` (panics on TRNG failure) | template |
| Many saved items | `Store::create_record(prefix, &mut trng, bytes)` + `Store::load_records(prefix, parse)` → `records::Loaded { items: (key, T, basis), unreadable }`; never overwrite unreadable keys | contacts, cipher |
| Settings next to records in one dict | name records with `records::new_key(prefix, ..)`; any key failing `records::is_key(prefix, name)` (e.g. `meta`) is skipped by `load_records` | decide, carse |
| One-time code for LAN transfer | TRNG one-time code, worker thread, 2-minute timeout, cancel key, 3-strike limit; README says plaintext | flashcards, writer |
| One settings blob | `appkit::store::Store::save/load` with a `version` field; read-only on parse error | template `src/app.rs` |
| Secrets in memory | `zeroize` on drop, redacted `Debug`, clear the screen on Background | keygen `core/src/secret.rs`, otp |
| Encoder correctness | reference vectors from the standard + differential tests against a crate in `[dev-dependencies]` only | qrcode (`qrcode` 0.12), barcode (reportlab vectors) |
| Drawing many cells/modules | `Screen::fill_rects(iter of (tl, br), color)` batches GAM calls | qrcode `src/ui.rs` |
| Dates, month grids, recurrence | `precursor_appkit_core::date` (`Date`, `YearMonth`, `add_months`, `month_grid`, weekdays; `serde` feature) | planner, budget |
| Today's date on device | `llio` local time → `Date`; fall back to the last date shown if the clock isn't set | planner `src/app.rs` |
| Real elapsed time (suspend, unset/changed clock) | `precursor_appkit_core::clock::Timekeeper::elapsed_secs(wall_s, mono_ms)` — carries sub-second remainders; persist `last_wall_s()` | pet |
| Seeded deterministic simulation | `entropy::SimRng::new(trng_seed)` (never for secrets) | fortress |
| Trig, rotation, polar points, symmetric steps | `precursor_appkit_core::fixed` (`sin`/`cos` Q16.16 over 4096-unit turns, `polar`, `rotate`, `step_angle`, `isqrt`) — never a local sine table (audit X1) | — (round 5) |
| Figures that may exceed the screen | `Screen::lines(segs, clip, width)` clips with `fixed::clip_line`; `Screen::circles` for real circles | — (round 5) |
| HP-style RPN, unary minus, overflow display | see calc `core/` (70 tests) | calc |
| Timers that alert in background | pump keeps ticking while an alert is armed, no redraws; time from ticktimer deltas + wall-clock gap check | timers |
| Text entry with a cursor | `precursor_appkit_core::text::TextField` (char-indexed cursor, max length, redacted Debug) | writer, qrcode |
| Word wrap / long text | `text::wrap(s, width)` and `text::Pager` (`label()` = "Page 2/5"); draw with `Screen::para` | otp, cipher |
| Key → app input | map `appkit::keys::Key` to a core `Input` enum in the shell; core tests drive `Input` | cipher, contacts `core/src/ui.rs` |

## Lint

- `manifest-locales` (added after the docs audit): every `menu_name` entry needs `en`, `en-tts`, `fr`, `ja`, `zh`; `locales/build.rs` panics on a missing language. Copy the English text for untranslated ones. App strings in `locales/i18n.json` follow the same rule.

- `#[cfg(test)] mod tests;` files and `0x1B` inside data tables used to be false positives; fixed in round 1a.

## Open (not yet automated)

- `Key`→`Input` mapping (every app writes one; probably fine as app code).
- Modal confirm/menu overlay, right-aligned text, non-blocking alert thread (round 2a).

## Log

- 2026-11-21 round 3b (mathdrill, zork, fortress, pet), Sonnet: 4/4 pass first submission, 36 min / 80 tool calls; tests 33/19/27/30. Orchestrator review found a time-truncation bug in pet's clock the gate missed → shared `clock::Timekeeper` (lesson 16) and `entropy::SimRng`. Open: `Screen::bar`, text-grid camera.

- 2026-11-21 round 3a (carse, decide, mines, flashcards), Sonnet: 4/4 pass first submission, 24 min / 87 tool calls; tests 46/48/28/27. Toolkit: appkit re-exports text/records/date, `Stuck(0)` documented, lessons 13–15, two patterns. Open: shared one-time-code TCP transfer helper (writer + flashcards), foreground `Clock`, pixel digit font.

- 2026-11-21 round 2b (othello, c64, gameboy) — first Sonnet round: 3/3 pass first submission, 12 min / 45 tool calls (othello and c64 resumed from WIP). Gate tests: 59/90/51. Toolkit: `-v` prints lint findings, template ignores `Cargo.lock`, lesson 11. Open: Key match helper where Backspace is input and F4 exits (c64 run mode).

- 2026-11-21 docs audit (`ecosystem/DOCS_AUDIT.md`, toolkit vs Xous Book + source): fixed 7 contradictions (`--app-feature`, `generated.rs`, `update_progress`, manifest locales, members note, `KeypadEnter` = `∴`, stale script name); documented i18n, `submenu`, IPC (try_send, deferred responses, memory messages, limits), ticktimer/susres, app-loader dev loop, `std::fs` PDDB, `menu_matic`, hosted-mode limits; new lint rule `manifest-locales`.

- 2026-11-20 round 1b (budget, planner): pass first submission; ~59 min for 2 apps (longest so far, machine heavily loaded). Toolkit: `date` module (was copied into both), `records::fresh_key` collision guard used by `Store::create_record`; lesson 10.

- 2026-11-20 round 2a (calc, writer, timers): all pass first submission; 46 min, 151 tool calls for 3 apps; ~/.cargo lock contention cost the most time. Toolkit: added `text::{TextField, wrap, Pager}`, `records`, `Store::{load_records, create_record}`, `Screen::{fill_rects, para}`; lessons 6–8.

- 2026-11-20 round 1a (barcode, qrcode, keygen, otp) and 1b part (cipher, contacts): all six pass the gate on first submission; ~20–26 min per two apps. Main rework came from toolkit gaps (copied entropy, lint false positives, toolchain drift), not from worker errors.
