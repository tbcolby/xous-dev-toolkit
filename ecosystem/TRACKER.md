# Ecosystem Build Tracker

> **The wave tables below record what the original pipeline claimed, not verified status.** The 2026-11-20 audit found none of these apps builds against xous-core at the anchor commit. Real status is the generated **Build status** section at the bottom; regenerate it with `bin/integrate --check --json status.json <repos>` then `bin/status status.json --update ecosystem/TRACKER.md`. Don't hand-edit status.

## Wave 1 — Quick Wins

| # | App | Repo | Port | Stage | Claimed |
|---|-----|------|------|-------|--------|
| 1 | QR Code Generator | precursor-qrcode | 7880 | 8/8 | claimed |
| 2 | Barcode Generator | precursor-barcode | 7881 | 8/8 | claimed |
| 3 | Decision Engine | precursor-decide | — | 8/8 | claimed |
| 4 | Minesweeper | precursor-mines | — | 8/8 | claimed |

## Wave 2 — Productivity Core

| # | App | Repo | Port | Stage | Claimed |
|---|-----|------|------|-------|--------|
| 5 | Contact Book | precursor-contacts | 7882 | 8/8 | claimed |
| 6 | Budget Ledger | precursor-budget | 7883 | 8/8 | claimed |
| 7 | Day Planner | precursor-planner | — | 8/8 | claimed |
| 8 | Math Drill | precursor-mathdrill | 7890 | 8/8 | claimed |

## Wave 3 — Crypto & Security

| # | App | Repo | Port | Stage | Claimed |
|---|-----|------|------|-------|--------|
| 9 | Cipher Workshop | precursor-cipher | 7887 | 8/8 | claimed |
| 10 | One-Time Pad | precursor-otp | 7888 | 8/8 | claimed |
| 11 | Key Ceremony | precursor-keygen | 7889 | 8/8 | claimed |

## Wave 4 — Visual Engines

| # | App | Repo | Port | Stage | Claimed |
|---|-----|------|------|-------|--------|
| 12 | Spirograph Engine | precursor-spirograph | — | 8/8 | claimed |
| 13 | Lissajous Machine | precursor-lissajous | — | 8/8 | claimed |
| 14 | Moire Generator | precursor-moire | — | 8/8 | claimed |
| 15 | Sacred Geometry | precursor-sacred | — | 8/8 | claimed |
| 16 | Kaleidoscope | precursor-kaleidoscope | — | 8/8 | claimed |
| 17 | Penrose Tiler | precursor-penrose | — | 8/8 | claimed |
| 18 | Fractal Flames | precursor-flames | — | 8/8 | claimed |
| 19 | Reaction-Diffusion | precursor-reaction | — | 8/8 | claimed |

## Wave 5 — Network & Parsing

| # | App | Repo | Port | Stage | Claimed |
|---|-----|------|------|-------|--------|
| 20 | Regex Tester | precursor-regex | 7891 | 8/8 | claimed |
| 21 | JSON Explorer | precursor-json | 7885 | 8/8 | claimed |
| 22 | RSS Reader | precursor-rss | 7884 | 8/8 | claimed |
| 23 | Git Log Viewer | precursor-gitlog | 7886 | 8/8 | claimed |
| 24 | Network Scanner | precursor-netscan | — | 8/8 | claimed |
| 25 | Cron Scheduler | precursor-cron | — | 8/8 | claimed |

## Wave 6 — Complex Simulations

| # | App | Repo | Port | Stage | Claimed |
|---|-----|------|------|-------|--------|
| 26 | Conway's Life | precursor-life | 7892 | 8/8 | claimed |
| 27 | Game of Death | precursor-death | — | 8/8 | claimed |
| 28 | SQLite Client | precursor-sqlite | 7895 | 8/8 | claimed |

## Wave 7 — The Legends

| # | App | Repo | Port | Stage | Claimed |
|---|-----|------|------|-------|--------|
| 29 | Text Adventure Engine | precursor-zork | 7893 | 8/8 | claimed |
| 30 | Tamagotchi | precursor-pet | — | 8/8 | claimed |
| 31 | Game Boy Emulator | precursor-gameboy | 7894 | 8/8 | claimed |
| 32 | Dwarf Fortress ASCII | precursor-fortress | — | 8/8 | claimed |

---

## Agent Evolution Log

| App # | New Agents Created | Existing Agents Updated | Key Learnings |
|-------|-------------------|------------------------|---------------|
| 1 | encoding.md | — | GF(2^8) from scratch works; JSON settings pattern validated; header/footer helpers reusable |
| 2 | — | encoding.md validated | App shell is a proven template; auto-format detection pattern established |
| 3 | randomness.md | encoding.md referenced | Multi-tool state machine; text input sub-state; TRNG wrapper; independent module pattern |
| 4 | — | randomness.md validated, graphics.md + architecture.md updated | Dynamic grid cell rendering; ticktimer integration; iterative flood fill |
| 5 | — | — | Scrollable list with filter; field edit sub-mode; confirmation dialog pattern |
| 6 | — | — | Cents-based currency; amount string parsing; summary dashboard pattern |
| 7 | — | — | Calendar month grid; Sakamoto's day-of-week; multi-field form; date arithmetic |
| 8 | — | randomness.md validated | Reverse-generated division; feedback auto-advance; progress bar; best score caching |
| 9 | — | encoding.md validated | Classic cipher module (pure functions); frequency analysis; key entry branching; reverse operation |
| 10 | — | randomness.md, encoding.md validated | Bulk TRNG bytes; XOR OTP with hex encoding; pad lifecycle tracking; hex I/O filtering |
| 11 | — | randomness.md, encoding.md validated | Diceware passphrase (~630 words); entropy estimation without float; base64 encoding; char class guarantee |
| 12 | — | graphics.md validated | Fixed-point 16.16 trig; parabolic sin/cos; hypotrochoid/epitrochoid parametric curves; precomputed point buffer |
| 13 | — | — | Lissajous parametric (sin(at+d), sin(bt)); LCM period; ratio table grid UI; phase in 1/1024 units |
| 14 | — | graphics.md validated | Bitmap XOR overlay; per-pixel pattern rendering; run-length display optimization; octant atan2 |
| 15 | — | — | Primitive enum (Line/Circle); polygon/star helpers; circle as 32-segment polyline; resizable figures |
| 16 | — | randomness.md validated | Dihedral symmetry (rotation+mirror); LCG PRNG for deterministic seeds; sector-based generation |
| 17 | — | — | Robinson triangle subdivision; golden ratio as 1618/1000; Penrose P2 aperiodic tiling; interactive level |
| 18 | — | randomness.md, encoding.md validated | Chaos game IFS; weighted transform selection; compile-time FP constants; 8 classic fractal presets |
| 19 | — | — | Gray-Scott reaction-diffusion; reduced grid with 4x upscaling; timer-driven auto-step; parameter space presets |
| 20 | — | encoding.md validated | Backtracking regex engine; character classes [a-z]; greedy quantifiers; cheat sheet UI |
| 21 | — | — | Recursive descent JSON parser; tree view with expand/collapse; auto-expand 2 levels; numbers stored as String |
| 22 | — | encoding.md validated | RSS 2.0 tag extraction; HTML stripping; entity decoding; embedded sample feeds |
| 23 | — | — | Git log line parser; case-insensitive search; commit list/detail navigation; 10 sample commits |
| 24 | — | — | CIDR subnet math (bitwise); 25-port reference table; simulated scan with 6 hosts; per-host detail |
| 25 | — | — | 5-field cron parser; alias resolution (@daily etc.); human-readable describe(); 10 presets + reference |
| 26 | — | graphics.md validated | B3/S23 toroidal grid; run-length cell rendering; 10 classic patterns (Gosper Gun); crosshair editor |
| 27 | — | — | Generalized B/S rule engine; 12 Life-like automata; rule notation display; shared grid architecture |
| 28 | — | encoding.md validated | In-memory SQL engine; tokenizer with quoted strings; WHERE comparisons; 3 sample tables; query history |
| 29 | — | — | 6-room dungeon world; 7 items; 9 verbs; partial-match parser; score tracking; scrollable output |
| 30 | — | randomness.md validated | 6 lifecycle stages; 5 stats; 6 actions; ASCII art sprites; tick-driven sim; stat bars |
| 31 | — | encoding.md validated | LR35902 CPU core; ~60 opcodes; register/flag display; disassembler; 4 demo programs; memory viewer |
| 32 | — | — | 42x28 ASCII map; 16 tile types; 7 dwarves with AI; 6 resources; mining/building/farming; seasonal calendar |

## Cumulative Stats

- Apps shipped: **32 / 32 — COMPLETE**
- Total LOC: ~38,500
- Specialist agents created: 2 (encoding.md, randomness.md)
- Toolkit updates: 32
- Screenshots captured: 0 (pending Renode)

## Ecosystem Complete

All 32 apps shipped across 7 waves. Every app follows the standard Precursor/Xous pattern:
- `no_std`/`no_main` Rust with GAM API
- Standard event loop (Redraw/Rawkeys/FocusChange/Quit)
- Consistent UI: header bar, footer hints, 336x536 monochrome
- Individual GitHub repos under tbcolby/
- Apache 2.0 license, README, CLAUDE.md, AGENTS.md

## Build status

<!-- status:begin -->
_Generated by `bin/status` on 2026-11-20 from a hosted check at xous-core `31fbe23` (anchor `31fbe23`). **0/40 build.**_

| App | Hosted build | Errors | Warnings | First error |
|---|---|---:|---:|---|
| barcode | ❌ fails | 93 | 0 | failed to resolve: use of unresolved module or unlinked crate `graphics_server`: use of unresolved module or u |
| budget | ❌ fails | 74 | 4 | failed to resolve: use of unresolved module or unlinked crate `alloc`: use of unresolved module or unlinked cr |
| c64 | ❌ fails | 1 | 0 | unresolved path dependencies ['../../xous-core/services/gam', '../../xous-core/services/pddb'] |
| calc | ❌ fails | 1 | 0 | call to unsafe function `destroy_server` is unsafe and requires unsafe function or block: call to unsafe funct |
| carse | ❌ fails | 2 | 0 | call to unsafe function `destroy_server` is unsafe and requires unsafe function or block: call to unsafe funct |
| cipher | ❌ fails | 1 | 0 | There are multiple `cipher` packages in your project, and the specification `cipher` is ambiguous. |
| contacts | ❌ fails | 72 | 4 | failed to resolve: use of unresolved module or unlinked crate `alloc`: use of unresolved module or unlinked cr |
| cron | ❌ fails | 44 | 1 | failed to resolve: could not find `RawkeysEvent` in `gam`: could not find `RawkeysEvent` in `gam` |
| death | ❌ fails | 38 | 0 | failed to resolve: could not find `RawkeysEvent` in `gam`: could not find `RawkeysEvent` in `gam` |
| decide | ❌ fails | 58 | 3 | failed to resolve: use of unresolved module or unlinked crate `alloc`: use of unresolved module or unlinked cr |
| flames | ❌ fails | 30 | 1 | failed to resolve: could not find `RawkeysEvent` in `gam`: could not find `RawkeysEvent` in `gam` |
| flashcards | ❌ fails | 2 | 0 | field `0` of struct `xous::SID` is private: private field |
| fortress | ❌ fails | 51 | 1 | failed to resolve: could not find `RawkeysEvent` in `gam`: could not find `RawkeysEvent` in `gam` |
| gameboy | ❌ fails | 44 | 2 | failed to resolve: could not find `RawkeysEvent` in `gam`: could not find `RawkeysEvent` in `gam` |
| gitlog | ❌ fails | 88 | 0 | failed to resolve: could not find `RawkeysEvent` in `gam`: could not find `RawkeysEvent` in `gam` |
| hwtools | ❌ fails | 4 | 1 | `pddb` does not live long enough: borrowed value does not live long enough |
| json-explorer | ❌ fails | 53 | 0 | failed to resolve: could not find `RawkeysEvent` in `gam`: could not find `RawkeysEvent` in `gam` |
| kaleidoscope | ❌ fails | 37 | 1 | failed to resolve: could not find `RawkeysEvent` in `gam`: could not find `RawkeysEvent` in `gam` |
| keygen | ❌ fails | 46 | 3 | failed to resolve: use of unresolved module or unlinked crate `graphics_server`: use of unresolved module or u |
| life | ❌ fails | 37 | 0 | failed to resolve: could not find `RawkeysEvent` in `gam`: could not find `RawkeysEvent` in `gam` |
| lissajous | ❌ fails | 64 | 2 | failed to resolve: could not find `RawkeysEvent` in `gam`: could not find `RawkeysEvent` in `gam` |
| mathdrill | ❌ fails | 52 | 1 | failed to resolve: use of unresolved module or unlinked crate `graphics_server`: use of unresolved module or u |
| mines | ❌ fails | 69 | 4 | failed to resolve: use of unresolved module or unlinked crate `graphics_server`: use of unresolved module or u |
| moire | ❌ fails | 39 | 2 | failed to resolve: could not find `RawkeysEvent` in `gam`: could not find `RawkeysEvent` in `gam` |
| netscan | ❌ fails | 50 | 1 | failed to resolve: could not find `RawkeysEvent` in `gam`: could not find `RawkeysEvent` in `gam` |
| othello | ❌ fails | 1 | 22 | call to unsafe function `destroy_server` is unsafe and requires unsafe function or block: call to unsafe funct |
| otp | ❌ fails | 41 | 2 | failed to resolve: use of unresolved module or unlinked crate `graphics_server`: use of unresolved module or u |
| penrose | ❌ fails | 30 | 1 | failed to resolve: could not find `RawkeysEvent` in `gam`: could not find `RawkeysEvent` in `gam` |
| pet | ❌ fails | 54 | 0 | failed to resolve: could not find `RawkeysEvent` in `gam`: could not find `RawkeysEvent` in `gam` |
| planner | ❌ fails | 46 | 1 | failed to resolve: use of unresolved module or unlinked crate `graphics_server`: use of unresolved module or u |
| qrcode | ❌ fails | 1 | 0 | There are multiple `qrcode` packages in your project, and the specification `qrcode` is ambiguous. |
| reaction | ❌ fails | 32 | 2 | failed to resolve: could not find `RawkeysEvent` in `gam`: could not find `RawkeysEvent` in `gam` |
| regex-tester | ❌ fails | 43 | 2 | failed to resolve: could not find `RawkeysEvent` in `gam`: could not find `RawkeysEvent` in `gam` |
| rss-reader | ❌ fails | 51 | 2 | failed to resolve: could not find `RawkeysEvent` in `gam`: could not find `RawkeysEvent` in `gam` |
| sacred | ❌ fails | 29 | 0 | failed to resolve: could not find `RawkeysEvent` in `gam`: could not find `RawkeysEvent` in `gam` |
| spirograph | ❌ fails | 38 | 3 | failed to resolve: could not find `RawkeysEvent` in `gam`: could not find `RawkeysEvent` in `gam` |
| sqlite | ❌ fails | 48 | 0 | failed to resolve: could not find `RawkeysEvent` in `gam`: could not find `RawkeysEvent` in `gam` |
| timers | ❌ fails | 1 | 0 | call to unsafe function `destroy_server` is unsafe and requires unsafe function or block: call to unsafe funct |
| writer | ❌ fails | 1 | 0 | call to unsafe function `destroy_server` is unsafe and requires unsafe function or block: call to unsafe funct |
| zork | ❌ fails | 27 | 0 | failed to resolve: could not find `RawkeysEvent` in `gam`: could not find `RawkeysEvent` in `gam` |

Packaging warnings:

- c64: unresolved path dependencies ['../../xous-core/services/gam', '../../xous-core/services/pddb']; not added to the workspace
- cipher: code uses gam::APP_NAME_CIPHER_WORKSHOP, but package implies APP_NAME_CIPHER
- qrcode: code uses gam::APP_NAME_QRCODE_APP, but package implies APP_NAME_QRCODE
<!-- status:end -->
