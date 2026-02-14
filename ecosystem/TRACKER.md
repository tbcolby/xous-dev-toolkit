# Ecosystem Build Tracker

## Wave 1 — Quick Wins

| # | App | Repo | Port | Stage | Status |
|---|-----|------|------|-------|--------|
| 1 | QR Code Generator | precursor-qrcode | 7880 | 8/8 | **SHIPPED** |
| 2 | Barcode Generator | precursor-barcode | 7881 | 8/8 | **SHIPPED** |
| 3 | Decision Engine | precursor-decide | — | 8/8 | **SHIPPED** |
| 4 | Minesweeper | precursor-mines | — | 8/8 | **SHIPPED** |

## Wave 2 — Productivity Core

| # | App | Repo | Port | Stage | Status |
|---|-----|------|------|-------|--------|
| 5 | Contact Book | precursor-contacts | 7882 | 8/8 | **SHIPPED** |
| 6 | Budget Ledger | precursor-budget | 7883 | 8/8 | **SHIPPED** |
| 7 | Day Planner | precursor-planner | — | 8/8 | **SHIPPED** |
| 8 | Math Drill | precursor-mathdrill | 7890 | 8/8 | **SHIPPED** |

## Wave 3 — Crypto & Security

| # | App | Repo | Port | Stage | Status |
|---|-----|------|------|-------|--------|
| 9 | Cipher Workshop | precursor-cipher | 7887 | 8/8 | **SHIPPED** |
| 10 | One-Time Pad | precursor-otp | 7888 | 8/8 | **SHIPPED** |
| 11 | Key Ceremony | precursor-keygen | 7889 | 8/8 | **SHIPPED** |

## Wave 4 — Visual Engines

| # | App | Repo | Port | Stage | Status |
|---|-----|------|------|-------|--------|
| 12 | Spirograph Engine | precursor-spirograph | — | 8/8 | **SHIPPED** |
| 13 | Lissajous Machine | precursor-lissajous | — | 8/8 | **SHIPPED** |
| 14 | Moire Generator | precursor-moire | — | 8/8 | **SHIPPED** |
| 15 | Sacred Geometry | precursor-sacred | — | 8/8 | **SHIPPED** |
| 16 | Kaleidoscope | precursor-kaleidoscope | — | 8/8 | **SHIPPED** |
| 17 | Penrose Tiler | precursor-penrose | — | 8/8 | **SHIPPED** |
| 18 | Fractal Flames | precursor-flames | — | 8/8 | **SHIPPED** |
| 19 | Reaction-Diffusion | precursor-reaction | — | 8/8 | **SHIPPED** |

## Wave 5 — Network & Parsing

| # | App | Repo | Port | Stage | Status |
|---|-----|------|------|-------|--------|
| 20 | Regex Tester | precursor-regex | 7891 | 8/8 | **SHIPPED** |
| 21 | JSON Explorer | precursor-json | 7885 | 8/8 | **SHIPPED** |
| 22 | RSS Reader | precursor-rss | 7884 | 8/8 | **SHIPPED** |
| 23 | Git Log Viewer | precursor-gitlog | 7886 | 8/8 | **SHIPPED** |
| 24 | Network Scanner | precursor-netscan | — | 8/8 | **SHIPPED** |
| 25 | Cron Scheduler | precursor-cron | — | 8/8 | **SHIPPED** |

## Wave 6 — Complex Simulations

| # | App | Repo | Port | Stage | Status |
|---|-----|------|------|-------|--------|
| 26 | Conway's Life | precursor-life | 7892 | 8/8 | **SHIPPED** |
| 27 | Game of Death | precursor-death | — | 8/8 | **SHIPPED** |
| 28 | SQLite Client | precursor-sqlite | 7895 | 8/8 | **SHIPPED** |

## Wave 7 — The Legends

| # | App | Repo | Port | Stage | Status |
|---|-----|------|------|-------|--------|
| 29 | Text Adventure Engine | precursor-zork | 7893 | 8/8 | **SHIPPED** |
| 30 | Tamagotchi | precursor-pet | — | 8/8 | **SHIPPED** |
| 31 | Game Boy Emulator | precursor-gameboy | 7894 | 8/8 | **SHIPPED** |
| 32 | Dwarf Fortress ASCII | precursor-fortress | — | 8/8 | **SHIPPED** |

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
