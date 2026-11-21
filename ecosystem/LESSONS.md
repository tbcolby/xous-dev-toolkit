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
9. **Run the gate early, not just at the end.** Gate failures found at the end cost a full rebuild cycle each. Run `bin/lint` and `core` tests continuously; run `--grade` after the shell compiles.

## Patterns (use these, don't reinvent)

| Need | Use | Reference |
|---|---|---|
| Randomness for secrets | `precursor_appkit_core::entropy` (fallible; `Trng` implements `Entropy`) | keygen `core/src/gen.rs` |
| Randomness for games | `appkit::rng::Trng::below/shuffle` (panics on TRNG failure) | template |
| Many saved items | `Store::create_record(prefix, &mut trng, bytes)` + `Store::load_records(prefix, parse)` → `records::Loaded { items: (key, T, basis), unreadable }`; never overwrite unreadable keys | contacts, cipher |
| One settings blob | `appkit::store::Store::save/load` with a `version` field; read-only on parse error | template `src/app.rs` |
| Secrets in memory | `zeroize` on drop, redacted `Debug`, clear the screen on Background | keygen `core/src/secret.rs`, otp |
| Encoder correctness | reference vectors from the standard + differential tests against a crate in `[dev-dependencies]` only | qrcode (`qrcode` 0.12), barcode (reportlab vectors) |
| Drawing many cells/modules | `Screen::fill_rects(iter of (tl, br), color)` batches GAM calls | qrcode `src/ui.rs` |
| HP-style RPN, unary minus, overflow display | see calc `core/` (70 tests) | calc |
| Timers that alert in background | pump keeps ticking while an alert is armed, no redraws; time from ticktimer deltas + wall-clock gap check | timers |
| Text entry with a cursor | `precursor_appkit_core::text::TextField` (char-indexed cursor, max length, redacted Debug) | writer, qrcode |
| Word wrap / long text | `text::wrap(s, width)` and `text::Pager` (`label()` = "Page 2/5"); draw with `Screen::para` | otp, cipher |
| Key → app input | map `appkit::keys::Key` to a core `Input` enum in the shell; core tests drive `Input` | cipher, contacts `core/src/ui.rs` |

## Lint

- `#[cfg(test)] mod tests;` files and `0x1B` inside data tables used to be false positives; fixed in round 1a.

## Open (not yet automated)

- `Key`→`Input` mapping (every app writes one; probably fine as app code).
- Modal confirm/menu overlay, right-aligned text, non-blocking alert thread (round 2a).

## Log

- 2026-11-20 round 2a (calc, writer, timers): all pass first submission; 46 min, 151 tool calls for 3 apps; ~/.cargo lock contention cost the most time. Toolkit: added `text::{TextField, wrap, Pager}`, `records`, `Store::{load_records, create_record}`, `Screen::{fill_rects, para}`; lessons 6–8.

- 2026-11-20 round 1a (barcode, qrcode, keygen, otp) and 1b part (cipher, contacts): all six pass the gate on first submission; ~20–26 min per two apps. Main rework came from toolkit gaps (copied entropy, lint false positives, toolchain drift), not from worker errors.
