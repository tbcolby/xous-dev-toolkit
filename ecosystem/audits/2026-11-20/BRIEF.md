# Precursor app portfolio audit: shared brief

Tyler builds Rust apps for the Precursor handheld (Xous OS: microkernel, message-passing servers, 336x536 1-bit display, PDDB encrypted plausibly-deniable storage, TRNG, physical keyboard, ~100MHz RISC-V, small heap). He wants a detailed **bug and improvement report**. You are auditing one slice of the portfolio. This is READ-ONLY review: do not modify the source repos.

## Locations
- Source repos (shallow GitHub clones, tbcolby/*): `SCRATCH/precursor/<repo>/`
- Each app expects to live at `xous-core/apps/<name>/` (deps like `path = "../../services/gam"`).
- An integrated build tree: `SCRATCH/xous-core/` (upstream betrusted-io/xous-core HEAD 31fbe23, 2026-11-17, xous 0.9.71) with all apps copied into `apps/`. Apps declare `xous = "0.9.69"`, so they were likely written against an older xous-core. **Treat API mismatches as "upstream drift" (portability/maintenance), not as proof the app is broken on Tyler's machine.** Tyler's primary working copies live on an external drive that isn't mounted; GitHub may lag.
- Compiler + clippy output per app (hosted mode): `SCRATCH/lint/<pkg>.txt` (app-only lines) and `<pkg>.raw` (full). Summary: `SCRATCH/lint/summary.txt`. Packaging anomalies already found: `SCRATCH/path_anomalies.txt`.
- Upstream reference for correct API usage: `SCRATCH/xous-core/services/{gam,pddb,modals,trng,ticktimer...}`, `SCRATCH/xous-core/apps/{hello,ball,vault,repl}`.
- Tooling context: `SCRATCH/precursor/xous-dev-toolkit/` (agents/, CLAUDE.md, discoveries.md) describes Tyler's patterns and known gotchas (e.g. SERVER_NAME vs APP_NAME, Unicode arrow keys U+2190–2193, 500ms key-hold in Renode).

SCRATCH = /private/tmp/claude-501/-/222625fb-1cb9-475b-aba1-974303a9b50f/scratchpad

Pre-observed compile-error themes (verify per app, don't just repeat): ~280 `graphics_server` used without being a declared dependency; `alloc` used without `extern crate alloc`; `gam::RawkeysEvent` missing; `xous::destroy_server` now unsafe; `PddbKey::set_len` missing; arg-count mismatches (E0061) on gam/modals APIs; a genuine borrow-check error in mines `minefield.rs:247`.

## What to look for (read every .rs file in your slice)
1. **Correctness bugs**: logic errors, off-by-one, integer overflow/underflow (u8/usize arithmetic, subtraction wrap), panics (`unwrap` on IPC/PDDB/parse paths, slicing/indexing out of range, divide by zero), borrow errors, infinite loops, wrong state transitions, emulator/parser/algorithm correctness vs spec.
2. **Xous-specific**: correct server registration & name conventions, message handling (unhandled opcodes, scalar vs memory messages, blocking calls on the main loop), focus change handling (redraw/suspend when backgrounded), GAM canvas/redraw discipline, PDDB usage (basis handling, key sync, error handling, data-format versioning, plausible deniability leaks), TRNG usage, ticktimer pumps that drain battery when not focused, heap use on a tiny device.
3. **Security** (especially for crypto/vault apps: keygen, otp, cipher, contacts, budget, planner, qrcode): RNG quality, key/secret zeroization, secrets in logs, timing, homebrew crypto presented as secure, PDDB secrets in wrong basis.
4. **Build/packaging hygiene**: undeclared deps, path layout inconsistencies, Cargo.toml metadata, version pinning, missing manifest/context-name docs, README vs reality, tests (do pure-logic modules have unit tests? do they run on host?).
5. **Improvements**: UX on 1-bit/e-ink, keyboard ergonomics, code duplication across apps (candidates for a shared crate), performance, missing features clearly implied by README.

## Report format
Write to `SCRATCH/reports/<slice>.md`. For each app:
- One-line verdict + health score (0–10).
- **Bugs**: table with Severity (Critical/High/Medium/Low), `file:line`, description, concrete failure scenario, suggested fix. Only include issues you verified by reading the code; mark anything uncertain as "Suspected".
- **Upstream drift**: concise summary of what breaks against current xous-core and the fix pattern (don't list hundreds of identical errors).
- **Improvements**: prioritized list.
End with a **cross-cutting** section for patterns repeated across your slice.

Your final message back should be a ≤250-word summary: top 5 most severe findings with file:line, and per-app scores. The full detail goes in the file.
