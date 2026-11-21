# Precursor apps: verified fact sheet and site claim check

Generated 2026-11-21 from /Users/tyler/Dev/precursor (current code and READMEs). Repo URL pattern: https://github.com/tbcolby/<dir>.

## 0. Headline facts

- **40 apps total = 39 `precursor-*` app repos + `carse-automata`** (added by the orchestrator; the first pass missed it because of its repo name). 40 directories match `precursor-*`, but only 39 of those are apps. `precursor-apps` is a superseded first-generation guidance repo (its CLAUDE.md says "no longer maintained", points to xous-dev-toolkit). It has no Cargo.toml, no tests.
- The 39 apps: productivity 9, security 3, devtools 9, games 6, emulators 2, generative-art 10.
- Host tests (`#[test]` under core/, libs/ and src/, excluding target/): **1,358 total** across the 39 `precursor-*` apps, **1,404 including carse-automata (46)**. One test in c64 (Klaus Dormann functional test) is `#[ignore]`-gated and is included in the count.
- Test-count caveat: the READMEs of four apps quote higher numbers than the `#[test]` count (cipher says 51, found 45; contacts 35 vs 29; keygen 42 vs 36; otp 42 vs 36). The gap is 6 each. Possibly tests from a shared crate that `cargo test` pulls in; not determined. Use the `#[test]` counts below, or say "about" if quoting.
- Every app README still says **"Renode capture not yet run"** (capture.py written, not executed) or "Not yet run on Renode or hardware". No app is verified on hardware or in Renode. The 10/10 gate (`bin/integrate --grade`) = hosted build + clippy + host tests + lint, nothing more.
- All 39 apps: Apache-2.0, author Tyler Colby / Colby's Data Movers, LLC, built on `precursor-appkit` from xous-dev-toolkit, data in PDDB (encrypted at rest by the PDDB; apps add no crypto of their own, README says so repeatedly).
- Network use across the whole set: **no app talks to the internet or any cloud.** Five apps use the LAN: c64, flashcards, gitlog, writer (inbound TCP listeners on 0.0.0.0, plaintext) and netscan (outbound TCP connect to hosts the user types). Everything else is offline. RSS Viewer does NOT fetch feeds.
- Source line count (all .rs files in the 39 repos, tests included): ~71,050.

## 1. Per-app sections

Format: dir | package | menu name (from manifest-fragment.json `en`) | version | tests | category. Source lines are in brackets only where relevant.

### Barcode
- dir `precursor-barcode`, package `barcode`, menu "Barcode", v0.2.0, **33 tests**, category **devtools**
- What: shows Code 128, Code 39, EAN-13 or UPC-A barcodes on the 1-bit screen so a scanner can read them off the display.
- Features: auto-detect format (13 digits EAN-13, 12 UPC-A, Code 39 charset, else Code 128) or fixed format via F1; check digits appended when omitted, wrong check digit reported not corrected; bars fit to screen (1-4 px per module, 40-300 px tall); saved codes with names in PDDB (`barcode.codes`, `barcode.settings`); text up to 80 chars; encoders tested against reportlab reference output.
- Limits: Renode capture not run; no barcode scanned off a real device; screen geometry unchecked; no Code 128 set A, Code 39 full-ASCII/mod-43, EAN-8, UPC-E, GS1-128; no network import.
- Network: none.

### Budget Ledger
- `precursor-budget`, package `budget`, menu "Budget Ledger", v0.2.0, **48 tests**, **productivity**
- What: personal income/expense ledger with a monthly summary.
- Features: income and expense with fixed categories (7 expense, 4 income); dated entries default to device clock date; monthly summary (income, expenses, net, spending by category, all-time balance); amounts are integer cents (u64, max 999,999,999.99), no floats; currency symbol setting ($ EUR GBP none); edit/delete.
- Limits: no CSV/export, no budgets/limits, no recurring entries, no search; categories fixed; without a set clock there is no "today"; delete is not secure erasure; Renode capture not run.
- Network: none.

### C64
- `precursor-c64`, package `c64`, menu "C64", v0.1.0, **91 tests** (README: "about 90"; ~4,300 lines of Rust), **emulators**
- What: hobby-grade Commodore 64 emulator for text-mode software and BASIC, not cycle-accurate, runs a fraction of the C64 library.
- Features: 6502 with all 151 legal opcodes, NMOS decimal mode, stable undocumented opcodes, page-cross penalties, IRQ; memory banking, 64 KB RAM; CIA1 keyboard matrix + timers A/B + jiffy IRQ; VIC-II text mode and standard hi-res bitmap only, raster counter derived from cycles; 50,000 cycles per 50 ms frame (`CYCLES_PER_FRAME = 50_000`, machine.rs:11); PRG launch and games/ROMs stored in PDDB (`c64.games`, `c64.roms`); F4 stops machine.
- Limits: no Commodore ROMs included (stand-in stub boots without BASIC; supply your own BASIC/KERNAL/chargen); no sprites, multicolour, extended-colour text, scroll registers, raster interrupts; no SID/sound, disk, tape, joystick, TOD, CIA2 timers; "most commercial games will not work"; speed on device untested; Renode capture not run; the earlier screenshots were removed because they showed Shellchat not the app (so **no c64 screenshots exist**).
- Network: **LAN, inbound TCP port 6464**, unauthenticated, plaintext, listens only while the Import screen is open; 5 s read timeout, 30 s whole-transfer, 64 KB cap. (`printf 'game.prg\n' | cat - game.prg | nc <ip> 6464`; `rom:basic|kernal|chargen` prefixes.)

### Calculator
- `precursor-calc`, package `calc`, menu "Calculator", v0.2.0, **70 tests**, **productivity**
- What: scientific calculator with algebraic and HP-style 4-level RPN modes, 64-bit float.
- Features: algebraic mode with precedence, right-assoc `^`, postfix `!`, implicit multiplication, `ans` continuation, scientific literals; RPN with X/Y/Z/T + LASTx and real HP stack lift; F1/F2/F3 menus (Math/Trig/More) with sqrt, ln, log, log2, exp, factorial/gamma, trig + inverse + hyperbolic, etc.; M0-M9 memory, DEG/RAD/GRAD, DEC/HEX/OCT/BIN display; history tape (algebraic only, last 20 saved); errors shown, never crash; exact trig at quarter turns.
- Keys (calculator README, "Keys" table): `M` toggle ALG/RPN, `A` cycle angle (DEG->RAD->GRAD, functions.rs:21-23), `B` cycle base (DEC->HEX->OCT->BIN, functions.rs:74-77), `S`+digit store, `K`+digit recall, `C` clear all, F1/F2/F3 menus, F4 leave. No Shift layer; upper-case = commands.
- Limits: HEX/OCT/BIN display-only, entry decimal; no complex numbers, units, solver, graphing, programmability; screenshots in `screenshots/` (19 PNGs) are from version 0.1 and show the old layout; Renode capture not run.
- Source: 11 .rs files, 3,030 lines including tests (core: calculator, format, functions, lib, menu, parser, rpn, tests; shell: main, app, ui). Parser is **recursive descent** (parser.rs), not shunting-yard.
- Network: none.

### Cipher Workshop
- `precursor-cipher`, package `cipher-workshop`, menu "Cipher Workshop", v0.2.0, **45 tests** (README says 51), **security** (educational)
- What: workshop for six classical ciphers with frequency analysis; explicitly "none of them protects anything".
- Features: Caesar, ROT13, Atbash, Vigenere (key up to 30 letters), substitution, rail fence (2-10 rails); encrypt/decrypt toggle; per-cipher weakness description; letter-frequency bar chart with Caesar guess; reverse check; save results to PDDB (plaintext and key stored next to ciphertext); key validation; buffers zeroized.
- Limits: learning only, no modern crypto, ASCII input up to 200 chars; saved messages hold plaintext and key; deletion is not secure erasure; Renode capture not run.
- Network: none.

### Contacts
- `precursor-contacts`, package `contacts`, menu "Contacts", v0.2.0, **29 tests** (README 35), **productivity**
- What: offline address book in the PDDB.
- Features: name (required, 40 chars), phone, email, notes, category (Personal/Work/Family/Other); as-you-type search across fields; sort A-Z or by category; add/edit/delete with confirmations; basis-aware (contacts in a locked PDDB basis disappear from the list).
- Limits: no vCard import/export, no QR export, no network, no sync; not secure erasure; Renode capture not run.
- Network: none.

### Cron Explainer
- `precursor-cron`, package `cron`, menu "Cron Explainer", v0.2.0, **54 tests**, **devtools**
- What: explains a 5-field cron expression in English and previews the next six fire times. **It does not schedule or run anything** (earlier name "Cron Scheduler" overstated it).
- Features: lists, ranges, steps, month/weekday names, @aliases (not @reboot); correct day-of-month/day-of-week OR rule; next six times with leap-year handling (searches 100 years); 17 samples and syntax card; optional typed start time.
- Limits: no L, W, #, ?, seconds or year field, wrapping ranges; no time zones/DST; nothing saved; no screenshots; Renode not run.
- Network: none.

### death
- `precursor-death`, package `death`, menu **"death"** (lower-case), v0.1.0, **17 tests**, **generative-art**
- What: "Game of Death", a Life-like cellular automaton explorer (not a game in the play sense).
- Features: 12 rule presets (Life, HighLife, Day & Night, Seeds, Diamoeba, Morley, Anneal, 2x2, Maze, Coagulations, Replicator, Stains) each with a viable start; custom B/S rule editor (toggles); cell editor; speed 50-1000 ms; 84x104 torus of 4 px cells; TRNG seeding (cosmetic).
- Limits: torus only; custom rules not saved; no text entry for rules; status line may be cut off; Renode not run.
- Network: none.

### Decision Engine
- `precursor-decide`, package `decide`, menu "Decision Engine", v0.2.0, **48 tests**, **games** (board-game companion)
- What: dice, coins, cards, 8-ball, spinner, scoreboard, turn tracker, tournament bracket; all randomness from hardware TRNG without modulo bias.
- Features: dice d4-d100, 1-20 dice, advantage/disadvantage, exploding, drop lowest; coin flip with custom sides; 52-card deck (+jokers); Magic 8-Ball (20 answers); spinner up to 12 weighted segments; scoreboard up to 8 players; turn tracker up to 20; single-elimination tournament 2-16 players; saved state in `decide.state`.
- Limits: dice history and deck not persisted; plain text output, no animations; "true random" = the OS TRNG, not tested; Renode not run.
- Network: none.

### IFS Fractals (dir "flames")
- `precursor-flames`, package `flames`, menu "IFS Fractals", v0.2.0, **16 tests**, **generative-art**
- What: chaos-game IFS attractors. **Not** Scott Draves fractal-flame (no variations or log-density colour); repo kept its name.
- Features: 8 presets (Barnsley Fern, Sierpinski Triangle and Carpet, Dragon, Koch Snowflake, Crystal, Maple Leaf, Tree); fit-to-screen with uniform scale; 10K-200K iterations in 4,000-iteration slices; TRNG seed, N for new seed.
- Limits: 1-bit points only; nothing saved; Renode not run.
- Network: none.

### Flashcards
- `precursor-flashcards`, package `flashcards`, menu "Flashcards", v0.2.0, **27 tests** (~1,720 lines), **productivity**
- What: multi-deck flashcards with spaced repetition; decks move in and out as TSV text over the LAN.
- Features: up to 500 cards/deck, 300 chars per side, demo deck "Xous Basics" on first run; browse (flip, next/prev, unbiased shuffle); **study mode uses Leitner boxes, intervals 0/1/3/7/16/35 days, grades Again/Good/Easy, due/total on deck list; explicitly "not SM-2", no ease factor**; TSV import and export (optional `#name:` line, `front<TAB>back`); delete.
- Keys: Space/Enter/F2 flip, Right/N/F3 next, Left/P previous, S shuffle, R study, I import, M manage, Q/Backspace/F4 leave, F1 help.
- Limits: transfers are plaintext on the LAN; text clipped at 300 chars/side, not paged; no per-card ease factor or statistics; due dates need the clock set; screenshots (5 PNGs) are from 0.1 and show the IME bar, "kept only as history"; Renode not run.
- Network: **LAN, inbound TCP, import port 7878, export port 7879** (xfer.rs:9-10), listener only after you press I/E, 2-minute limit, **six-digit one-time code** from TRNG sent as first line, 3 wrong codes end it, 64 KB import cap, **not encrypted**.

### Fortress
- `precursor-fortress`, package `fortress`, menu "Fortress", v0.2.0, **27 tests**, **games**
- What: small fortress-building management sim inspired by Dwarf Fortress (borrows nothing from it).
- Features: 40x30 text map; 7 dwarves; dig orders; jobs Mining/Farming/Woodcutting/Idle; build bed, table, chair, door, barrel, wall, anvil; smelt ore, brew drinks; seasons, migrants (up to 12 dwarves), hunger/sleep needs; one tick = 2 s while app is in front; autosave to PDDB (every 600 ticks, on leave).
- Limits: dwarves do not haul, build or fight; no z-levels/water/combat; furniture only affects score; nothing runs while app is in the background; layout unchecked; Renode not run.
- Network: none.

### Game Boy
- `precursor-gameboy`, package `gameboy`, menu "Game Boy", v0.1.0, **51 tests**, **emulators** (CPU debugger only)
- What: step debugger for the Sharp LR35902 CPU with 64 KB flat memory. **Not a Game Boy emulator**: no cartridge loading, screen, timer, joypad, sound.
- Features: all defined base opcodes and all 256 CB opcodes; flags, DAA; interrupts (IME with EI delay, IF/IE, 5 vectors, RETI); HALT; 11 illegal opcodes fault visibly; disassembler; five built-in demos (Counter, Add Two Numbers, Fibonacci, Memory Fill, BCD Add); memory viewer.
- Limits: no ROM loading, no MBC, no PPU/timers/serial; HALT bug and STOP not modelled; not run against Blargg cpu_instrs; Renode not run.
- Network: none.

### Git Log Viewer
- `precursor-gitlog`, package `gitlog`, menu "Git Log Viewer", v0.2.0, **38 tests**, **devtools**
- What: browse `git log` output on the device. **No git on the device**; a log arrives only by LAN push; otherwise a built-in sample of invented commits ("(sample)" in title).
- Features: parses default, `--format=fuller`, `--stat`, `-p` (stat/diff skipped) and `--oneline`; list, detail screen with full message, refs, parents; case-insensitive filter; up to 2,000 commits / 64 KB; imported log kept in PDDB (`gitlog`/`log`).
- Limits: dates reformatted, time zone dropped; `--graph` and custom `--pretty` formats not parsed; no diffs shown; Renode not run, no screenshots.
- Network: **LAN, inbound TCP port 7880** (xfer.rs:9), started with F1/`i`, 2-minute limit, 6-digit code, 3 wrong codes end it, 64 KB cap, **plaintext**.

### HW Tools
- `precursor-hwtools`, package `hwtools`, menu "HW Tools", v0.2.0, **30 tests**, **devtools**
- What: hardware readouts: battery, EC, SoC, ADC rails, the two readable GPIO pins, UART mux.
- Features: System tab (read-only, refresh every 2 s while in front; SoC DNA masked until `d`); GPIO tab (pins 2 and 5 readable via ADC; switching a pin to output needs confirmation and is reverted on exit); UART mux selector (Kernel/Log/Application, each confirmed, reverted on exit); only `last_tab` is stored.
- Limits: in Renode most readings are stubbed placeholders, meaningful only on real hardware; GPIO voltage scale (3.3 V) not verified against dividers; cannot set output level or read the other six pins (no llio wrappers); UART mux does not capture or send serial data; temperature/accelerometer raw, uncalibrated; Renode not run.
- Network: none.

### JSON Viewer
- `precursor-json`, package `json-viewer`, menu "JSON Viewer", v0.2.0, **38 tests**, **devtools**
- What: strict RFC 8259 JSON validator and collapsible tree viewer. "No network access."
- Features: strict parser (numbers, escapes, surrogate pairs, trailing commas rejected); depth limit 64; numbers kept as text; collapsible tree with path (`$.user.roles[1]`); full-value and 2-space pretty view, paged; error line/column/index with cursor placed; 7 samples; last input kept.
- Limits: typing is the only input (4,000 chars; no paste, file import or export); no JSON Pointer queries, editing or schema validation; Renode not run.
- Network: none.

### Kaleidoscope
- `precursor-kaleidoscope`, package `kaleidoscope`, menu "Kaleidoscope", v0.2.0, **13 tests**, **generative-art**
- What: kaleidoscope pattern generator, random line seeds in one wedge repeated with N-fold rotation and mirror symmetry.
- Features: 3-16 folds; 3-40 seed segments, reproducible from fold/shape/seed; TRNG random pattern on request; 10 presets (Snowflake, Crystal, Mandala, Butterfly, Star Burst, Web, Cathedral, Quilt, Frost, Chaos).
- Limits: plain 1 px line segments only; presets look different from 0.1; nothing saved; Renode not run.
- Network: none.

### Key Ceremony
- `precursor-keygen`, package `keygen`, menu "Key Ceremony", v0.2.0, **36 tests** (README 42), **security**
- What: password, PIN, passphrase, BIP-39 mnemonic and key-material generator on the hardware TRNG.
- Features: exact entropy figure (log2 of equally likely values, rounded down to 0.1 bit) with WEAK/MODERATE/GOOD/STRONG/EXCELLENT labels; rejection sampling (no modulo bias); fails closed on TRNG error; BIP-39 checked against reference vectors; save to vault with label (duplicate refused); saved list never shows values; secrets leave screen on focus loss and after 2 minutes idle.
- Limits: randomness is the OS TRNG server (app cannot test the noise source); values protected by PDDB encryption only; zeroize is best effort; delete is not secure erasure; Renode not run.
- Network: none.

### life
- `precursor-life`, package `life`, menu **"life"** (lower-case), v0.1.0, **16 tests**, **generative-art**
- What: Conway's Game of Life, B3/S23, 84x104 grid of 4 px cells.
- Features: 10 patterns (Glider, Blinker, Toad, Beacon, Pulsar, LWSS, R-pentomino, Diehard, Acorn, Gosper Gun) plus Random (25%, TRNG) and Empty; switchable edge rule (W): wrap or dead edge; cell editor; speed 50-1000 ms; redraws only changed cells.
- Limits: nothing saved; 4 px cells only; status line may be cut off; Renode not run.
- Network: none.

### Lissajous
- `precursor-lissajous`, package `lissajous`, menu "Lissajous", v0.1.0, **13 tests**, **generative-art**
- What: animated-pen Lissajous curves.
- Features: frequencies A,B 1-9, phase in 16 steps of 22.5 degrees; 10 presets; 9x9 ratio table of 81 thumbnails; integer-only Q16.16 maths; closes after one turn via gcd.
- Limits: equal amplitude on both axes; table redraw is heavy (~160 GAM messages); nothing saved; Renode not run.
- Network: none.

### Math Drill
- `precursor-mathdrill`, package `mathdrill`, menu "Math Drill", v0.2.0, **33 tests**, **productivity**
- What: timed 10-problem arithmetic quiz with saved bests.
- Features: add, subtract, multiply, divide, mixed; Easy (1-9), Medium (2-19), Hard (2-49); TRNG-drawn problems, exact integer answers; scoring 10/20/30 base + speed bonus (0 after 10 s) + 5 at streak 3; best per (operation, level) tracked per metric; clock pauses in background.
- Limits: quitting a session records nothing; layout unchecked; Renode not run.
- Network: none.

### Mines
- `precursor-mines`, package `mines`, menu "Mines", v0.2.0, **28 tests**, **games**
- What: Minesweeper with TRNG mine placement.
- Features: Beginner 9x9/10, Intermediate 16x16/40, Expert 16x20/60; first click and its 8 neighbours safe; iterative flood fill, flags, counter; clock counts only while app in front; best time per difficulty saved; double-press to abandon.
- Limits: no chording, no save/resume of a game; Expert cells ~21 px; Renode not run.
- Network: none.

### Moire Generator
- `precursor-moire`, package `moire`, menu "Moire Generator", v0.2.0, **24 tests**, **generative-art**
- What: two configurable layers XORed on the 1-bit display to show moire beats.
- Features: layer kinds Lines, Circles, Radial, Grid; spacing 3-30, angle, centre offsets up to 150 px; 8 presets; live tuning of layer B (angle +/-1 degree, spacing); integer-only maths.
- Limits: scattered patterns slow (Concentric Ripple ~19,000 rectangles, a few seconds per redraw); no animation; approximate atan (<0.25 degrees); nothing saved; Renode not run.
- Network: none.

### Net Check (dir "netscan")
- `precursor-netscan`, package `netscan`, menu "Net Check", v0.2.0, **34 tests**, **devtools**
- What: subnet calculator, 29-entry port reference and a small rate-limited TCP connect check. **"This is not a network scanner"**: no ping, ARP, raw packets, range sweep or fingerprinting; the old fake-host output is gone.
- Features: TCP connect check of one host (IPv4 or name) and at most 16 ports, only after a confirmation screen; 2 s timeout, 250 ms between attempts, cancellable; results open/closed/no reply/unreachable/error; nothing sent after connect; subnet calculator (mask, wildcard, network, broadcast, hosts, class, type incl. private/CGNAT/etc., /31 and /32 handled).
- Limits: no ICMP, ARP, UDP, IPv6, banner grabbing; connect check untested on a device with Wi-Fi (tested on loopback and a fake network); Renode not run.
- Network: **outbound TCP connect and DNS lookup to the host the user types** (LAN or any address; a public address gets a warning note). Nothing runs without the user pressing Enter.

### Othello
- `precursor-othello`, package `othello`, menu "Othello", v0.1.0, **59 tests** (in libs/othello-core), **games**
- What: Othello/Reversi with a CPU opponent.
- Features: bitboard engine, perft-verified move generation (4, 12, 56, 244, 1396, 8200); vs CPU at Easy/Medium/Hard/Expert (negamax alpha-beta, iterative deepening, exact endgame solver for last 8/10 squares, time limits 2/4/8/15 s, background thread); two-player mode; hint, undo, resign, history; save/resume one game; stats per level; "What if" review of finished games; colour chosen by TRNG.
- Limits: CPU strength untimed on a Precursor and likely shallower than labelled; no opening book, no transposition table; no sound/animations; TCP game export from earlier versions removed; scoring counts discs only; Renode not run.
- Network: none.

### One-Time Pad
- `precursor-otp`, package `otp`, menu "One-Time Pad", v0.2.0, **36 tests** (README 42), **security**
- What: two-party one-time pad: generate pads from the TRNG, share by reading hex, encrypt/decrypt short text by XOR. Not HOTP/TOTP.
- Features: pads 16-512 bytes; Forward/Reverse roles so two devices do not reuse bytes; used spans recorded and synced before output is shown; reuse warnings on decrypt; pad status FRESH/IN USE/SPENT; fail-closed on TRNG error; sensitive screens wiped on leave, focus loss, 2 min idle.
- Limits: **no integrity/MAC** (bit-flipping possible); ciphertext length reveals message length; pad sharing is by typing hex (no QR/USB); only safe if the other party also uses this rule; zeroize best effort; Renode not run.
- Network: none.

### Penrose Tiler
- `precursor-penrose`, package `penrose`, menu "Penrose Tiler", v0.2.0, **13 tests**, **generative-art**
- What: Penrose P3 rhomb tilings by Robinson triangle deflation.
- Features: seeds Decagon and Star; levels 1-7 (star max 6); exact triangle counts tested (decagon 10, 20, 50, 130, 340, 890, 2,330, 6,100); rhomb edges only; integer 16.16 maths.
- Limits: level 7 computed in one go; not zoomable/pannable; outline is a decagon or star, not a patch of an infinite tiling; Renode not run.
- Network: none.

### Pet
- `precursor-pet`, package `pet`, menu "Pet", v0.1.0 in Cargo (README describes 0.2-style audit), **24 tests**, **games**
- What: real-time virtual pet that keeps ageing while the app is closed.
- Features: catch-up by elapsed RTC/monotonic seconds (cap 14 days); life stages egg (5 min) to senior (21 days) then dies of old age; feed, play, sleep, medicine, clean, discipline; cannot die of neglect while the app is not running (health floors at 1) but can on screen; best lifespan remembered; one JSON record in PDDB.
- Limits: time rules are a design choice, not Tamagotchi canon; without a set RTC suspend/reboot time is not counted; no sound or vibration alerts; layout unchecked; Renode not run.
- Network: none.

### Day Planner
- `precursor-planner`, package `planner`, menu "Day Planner", v0.2.0, **48 tests**, **productivity**
- What: day planner with events, month calendar and tasks.
- Features: day view; Monday-first month view with event dots; events with time (5-min steps) or all-day, priority, repeat none/daily/weekly/monthly/yearly; tasks with done and priority; edit and delete with confirmation.
- Limits: no reminders or alarms (does nothing in background); editing/deleting a repeating event changes every occurrence; no agenda view, search, import/export; "today" needs a set clock; Renode not run.
- Network: none.

### QR Code
- `precursor-qrcode`, package `qrcode-app`, menu "QR Code", v0.2.0, **26 tests**, **devtools**
- What: QR code generator for the 1-bit display so a phone can read text off the device.
- Features: versions 1-10 (21x21 to 57x57), EC levels L/M/Q/H, numeric/alphanumeric/byte mode, all 8 masks scored per ISO/IEC 18004; capacity at EC-L 652 digits / 395 alphanumeric / 271 bytes; live fit counter; cursor editing; templates (F1 WiFi, F2 URL, F3 vCard 3.0); module size auto or 2-8 px; saved codes; hide on focus loss. Tested against the independent `qrcode` crate for every version/EC/mode/mask combination.
- Limits: no versions above 10, mixed-mode, Kanji, ECI, Micro QR; WiFi template not escaped for you; any camera in view can read the code; no network import; no code scanned off a real device; Renode not run.
- Network: none.

### reaction
- `precursor-reaction`, package `reaction`, menu **"reaction"** (lower-case), v0.1.0, **13 tests**, **generative-art**
- What: Gray-Scott reaction-diffusion simulator on an 84x104 torus.
- Features: 8 presets (spots, stripes, mazes, Worms, Waves...) retuned by tests; Q0.16 fixed-point, ~70 KB, no per-step allocation; 5-50 steps/frame capped ~150 ms; TRNG-seeded blobs; changed-cell redraw.
- Limits: frame rate unmeasured on hardware; slow presets need thousands of steps; hard threshold, no dithering; nothing saved; Renode not run.
- Network: none.

### Regex Lab
- `precursor-regex`, package `regex-lab`, menu "Regex Lab", v0.2.0, **35 tests**, **devtools**
- What: regular-expression workbench with live validation and match list. "No network access."
- Features: live syntax errors with position; memoised matcher (no exponential blowup) plus 500,000-step budget; all matches with `[start..end]` and context; ignore-case (F2); syntax reference (F1); matching compared with the Rust `regex` crate on thousands of generated patterns; state kept in PDDB.
- Limits: no backreferences, lookaround, named groups, inline flags, POSIX classes, Unicode properties; groups do not capture; no replace; single-line text only; `\d \w \s \b` ASCII; pattern 120 chars, text 500 chars typed; Renode not run.
- Network: none.

### RSS Viewer
- `precursor-rss`, package `rss-viewer`, menu "RSS Viewer", v0.2.0, **38 tests**, **productivity**
- What: reader for RSS/Atom XML. **It does not fetch anything** (no network, DNS or TLS code); you type XML in, reopen the last typed feed, or use four built-in sample feeds with invented headlines.
- Features: RSS 2.0, RSS 1.0 items and Atom; front-to-back parser, panic-free on garbage; HTML-to-text with entity decoding; scrolling item list, paged articles; first 100 items kept.
- Limits: typing is the only way in (8,000 chars, no paste/import/export); no read state, favourites or search; not an XML validator; Renode not run.
- Network: none (explicitly, no fetching).

### Sacred Geometry
- `precursor-sacred`, package `sacred`, menu "Sacred Geometry", v0.2.0, **18 tests**, **generative-art**
- What: ten compass-and-straightedge figures drawn as real circles and lines.
- Features: Seed of Life, Flower of Life, Metatron's Cube, Sri Yantra (simplified), Vesica Piscis, Torus of Life (simplified), Hexagram, Pentagram, Golden Spiral, Enneagram {9/4}; auto-fit radius, resize up to 2x; Left/Right between figures; Q16.16 trig, no float at runtime.
- Limits: large circles slow (GAM visits every pixel of the bounding square; circles >700 px skipped); Sri Yantra, Torus and Enneagram are simplified and labelled so; 1 px lines; Renode not run.
- Network: none.

### spirograph
- `precursor-spirograph`, package `spirograph`, menu **"spirograph"** (lower-case), v0.1.0, **11 tests**, **generative-art**
- What: animated-pen hypotrochoid and epitrochoid curves.
- Features: outer radius R 10-200, inner r, pen distance d 5-100, steps of 5; 10 presets (Classic Star, Pentagon Flower, Triangle Rose, Seven-Point, Cardioid, Limacon, Nephroid, Astroid, Deltoid, Spirograph #42); every curve closes (gcd-based, 256-6144 segments); integer maths.
- Limits: high-lobe curves hit the 6144-segment cap and take seconds; some epitrochoids fill slightly less than the box; nothing saved; Renode not run.
- Network: none.

### SQL Explorer
- `precursor-sqlite`, package `sql-explorer`, menu "SQL Explorer", v0.2.0, **39 tests**, **devtools**
- What: in-memory SQL playground. **Not SQLite**: no SQLite files, no compatibility promise, tables vanish when the app exits.
- Features: SELECT (columns, COUNT(*), WHERE, ORDER BY, LIMIT), INSERT, UPDATE, DELETE, CREATE/DROP TABLE, SHOW TABLES, DESCRIBE; exact decimals (no floats); LIKE, IS NULL, AND/OR/NOT; sample tables users/products/orders; scrollable results table; history of last 20 successful queries kept in PDDB; in-app help.
- Limits: no JOIN, GROUP BY, HAVING, DISTINCT, OFFSET, subqueries, UNION, aliases, functions other than COUNT(*), constraints, ALTER, indexes, transactions; 16 tables, 16 columns, 500 rows, 300-char queries; Renode not run.
- Network: none.

### Timers
- `precursor-timers`, package `timers`, menu "Timers", **v0.3.0**, **41 tests** (~2,290 lines), **productivity**
- What: Pomodoro, stopwatch with laps, and a list of named countdowns; all three can run at once and keep running when you leave the app.
- Features: Pomodoro work/short/long break with defaults **25 / 5 / 15 min and a long break after 4 work sessions** (pomodoro.rs:22), each 1-180 min, sessions 1-12, next phase starts by itself; stopwatch in tenths of a second with up to **99 laps** (timer.rs:85); up to **20 named countdowns**, 1 s to 24 h (countdown.rs:7-10), one runs at a time; **background alerts by vibration and/or notification** (1 s tick while a Pomodoro/countdown is armed); suspend-safe timing; settings persisted (`timers`/`settings`).
- Keys: F1 menu, F2/Enter start-pause, F3/`r` reset or restart, `l` lap, `s` settings, F4/Backspace back/leave.
- Limits: **no sound** (the audio toggle was removed because it did nothing); alerts up to ~1 s late; running timers not saved (a reboot stops them); setting the clock forward while a timer runs counts as suspended time; no export of laps; screenshots (4 PNGs) are from 0.2 and show the old layout; Renode not run.
- Network: none.

### Writer
- `precursor-writer`, package `writer`, menu "Writer", **v0.3.0**, **79 tests** (~3,730 lines), **productivity**
- What: markdown editor, one-entry-per-day journal, and forward-only typewriter mode.
- Features: documents (create, open, rename, delete with confirm), soft-wrapped monospace editing, optional line numbers, word count, UTF-8-safe cursor; preview renders headings, lists, block quotes, fenced code, rules line by line (inline `**bold**`, backticks and links shown as plain text); journal keyed by date from the device clock, previous/next/today, search across entries (up to 50 matching lines), refuses to open if clock unset; typewriter append-only with a draft that survives leaving; **export by TCP or by USB-keyboard autotype**; settings (autosave, line numbers).
- Keys (v0.3): **F1** menu, **F2** preview on/off, **F3** save, **F4** back (asks y/n/cancel if unsaved). Esc-prefix commands (Esc+[, Esc+], Esc+t, Esc+/) were removed because the device has no Esc key.
- Storage: dictionaries `writer.docs` (keys `_index`, `doc_<name>`), `writer.journal` (`_index`, `YYYY-MM-DD`), `writer.settings` (`config`, `typewriter_draft`); JSON records. Document name max 40 chars. **No 32 KB document limit exists in the code** (grep of core/src and src found none).
- Limits: no undo, no selection/copy/paste, no Home/End; inline markdown not styled; each key redraws the whole screen; TCP export is plaintext; screenshots (6 PNGs) are from 0.2 and show the old layout; Renode not run.
- Network: **LAN, inbound TCP port 7879** (export.rs:8) only during an export: one-time six-digit code sent first (`echo 123456 | nc <ip> 7879 > doc.md`), 3 wrong codes or 120 s (`TIMEOUT_SECS`) end it, F4 cancels, **text travels unencrypted**. USB autotype is the non-network alternative.

### Text Adventure (dir "zork")
- `precursor-zork`, package `zork`, menu **"Text Adventure"**, v0.2.0, **19 tests**, **games**
- What: a small original six-room text adventure. **Not Infocom's Zork** (no Zork map, text or puzzles); repo name kept.
- Features: find four treasures (spellbook, crown, gold coins, crystal orb) and bank them in the trophy case; two puzzles (brass key for the trapdoor, silver sword for cobwebs); score out of 75; parser ignores articles, takes partial words, `take all`; commands N S E W U D, look, examine, take, drop, put in case, unlock, cut, inventory, score, save/restore/restart/quit; autosave plus `slot1`.
- Limits: one map, six rooms, no NPCs, no light/dark; transcript keeps 300 lines; layout unchecked; Renode not run.
- Network: none.

### Carse Automata (repo `carse-automata`, not `precursor-*`)
- repo https://github.com/tbcolby/carse-automata, package `carse`, menu "Carse Automata", **46 tests**, category **games** (or generative-art)
- What: a cellular-automaton sandbox with no score or win state: a 64x64 toroidal grid of seven energy states evolves under hidden rules in finite "epochs"; you move a cursor, touch cells, and hold Space to let time run.
- Inspiration (README): James P. Carse, *Finite and Infinite Games*; Stephen Wolfram, *A New Kind of Science*.
- Limits: not run on Renode or hardware; README screenshots are from the 0.1 build (before the cell-size, cursor and diff fixes).
- Network: none.

### precursor-apps (not an app)
- Superseded first-generation guidance repo, no Cargo.toml. Contents: CLAUDE.md (a "superseded" notice), Precursor_Agent_Bootstrap.md, reference/discoveries.md, scripts/new-app.sh. Its old template had wrong key tables (Apple PUA arrows etc.). Do not list as an app.

## 2. Summary table

| dir | menu name | cat | tests | network |
|---|---|---|---|---|
| barcode | Barcode | devtools | 33 | none |
| budget | Budget Ledger | productivity | 48 | none |
| c64 | C64 | emulators | 91 | LAN in :6464, no auth, plaintext |
| calc | Calculator | productivity | 70 | none |
| cipher | Cipher Workshop | security | 45 | none |
| contacts | Contacts | productivity | 29 | none |
| cron | Cron Explainer | devtools | 54 | none |
| death | death | generative-art | 17 | none |
| decide | Decision Engine | games | 48 | none |
| flames | IFS Fractals | generative-art | 16 | none |
| flashcards | Flashcards | productivity | 27 | LAN in :7878 import / :7879 export, 6-digit code, plaintext |
| fortress | Fortress | games | 27 | none |
| gameboy | Game Boy | emulators | 51 | none |
| gitlog | Git Log Viewer | devtools | 38 | LAN in :7880, 6-digit code, plaintext |
| hwtools | HW Tools | devtools | 30 | none |
| json | JSON Viewer | devtools | 38 | none |
| kaleidoscope | Kaleidoscope | generative-art | 13 | none |
| keygen | Key Ceremony | security | 36 | none |
| life | life | generative-art | 16 | none |
| lissajous | Lissajous | generative-art | 13 | none |
| mathdrill | Math Drill | productivity | 33 | none |
| mines | Mines | games | 28 | none |
| moire | Moire Generator | generative-art | 24 | none |
| netscan | Net Check | devtools | 34 | outbound TCP connect + DNS to user-typed host |
| othello | Othello | games | 59 | none |
| otp | One-Time Pad | security | 36 | none |
| penrose | Penrose Tiler | generative-art | 13 | none |
| pet | Pet | games | 24 | none |
| planner | Day Planner | productivity | 48 | none |
| qrcode | QR Code | devtools | 26 | none |
| reaction | reaction | generative-art | 13 | none |
| regex | Regex Lab | devtools | 35 | none |
| rss | RSS Viewer | productivity | 38 | none (no fetching) |
| sacred | Sacred Geometry | generative-art | 18 | none |
| spirograph | spirograph | generative-art | 11 | none |
| sqlite | SQL Explorer | devtools | 39 | none |
| timers | Timers | productivity | 41 | none |
| writer | Writer | productivity | 79 | LAN in :7879 export, 6-digit code, plaintext |
| zork | Text Adventure | games | 19 | none |

Category counts: productivity 9 (budget, calc, contacts, flashcards, mathdrill, planner, rss, timers, writer); security 3 (cipher, keygen, otp); devtools 9 (barcode, cron, gitlog, hwtools, json, netscan, qrcode, regex, sqlite); games 6 (decide, fortress, mines, othello, pet, zork); emulators 2 (c64, gameboy); generative-art 10 (death, flames, kaleidoscope, life, lissajous, moire, penrose, reaction, sacred, spirograph). Total 39. Categories are my assignment; the repos do not carry a category field. Borderline: barcode/qrcode (could be productivity), decide (games vs productivity), life/death (simulation vs generative-art), gameboy (listed as emulator but is only a CPU debugger).

Package-name oddities (matter for `cargo -p` and links, not for the site): cipher -> `cipher-workshop`, json -> `json-viewer`, qrcode -> `qrcode-app`, regex -> `regex-lab`, rss -> `rss-viewer`, sqlite -> `sql-explorer`.

## 3. Claim check

Verdicts: TRUE, FALSE, CHANGED (was true of old version or is true with a materially different detail), UNVERIFIABLE (no source in repos/toolkit). Evidence is relative to /Users/tyler/Dev/precursor.

### 3a. /Users/tyler/colbysdatamovers.com/precursor-apps.html

| # | Claim | Current truth (evidence) | Verdict |
|---|---|---|---|
| A1 | "6 Apps Published" stat | 39 app repos exist. The page itself lists only 5 apps (Timers, Writer, Flashcards, C64, Calculator), so "6" is wrong even internally | FALSE |
| A2 | "100% Offline Capable" / "designed to work completely offline" | Core function is offline, but flashcards (7878/7879), writer export (7879), c64 import (6464), gitlog import (7880) open LAN listeners, and netscan makes outbound TCP connects (READMEs). All opt-in, none use the internet | CHANGED (say "works offline; optional LAN transfer") |
| A3 | "0 Cloud Dependencies" | No app has any cloud/internet code; RSS Viewer explicitly has no fetch (rss README) | TRUE |
| A4 | "Language: Rust" / "All apps built in Rust" | All app code is Rust; capture.py per app is Python tooling | TRUE |
| A5 | Toolkit: "Headless Renode, telnet monitor automation for macOS ARM64" | xous-dev-toolkit/scripts/renode_lib.py:768-790 starts `renode --disable-xwt -P <port>` and drives the monitor | TRUE |
| A6 | Command `renode --disable-xwt -P 4567 -e 'path add @xous-core; i @emulation/xous-release.resc; start'` | Closest real call: `[renode, "--disable-xwt", "-P", port, "-e", 'path add @"<root>"; i @<resc>']` (renode_lib.py:790-791). No trailing `start` in the launch; `start` is only used after timed key presses (renode_lib.py:510). Port default 4567 (scripts/README.md:17) | CHANGED |
| A7 | `python3 scripts/renode_interact.py screenshot output.png` | `scripts/renode_interact.py` **does not exist**. Deleted in commits 193dbdc / 1cf105c. Screenshot is now `ctl.screenshot(path)` in `renode_lib.py` driven via `renode_capture.py` | FALSE |
| A8 | `renode_interact.py init-pddb a` | No such command. Replaced by `python3 scripts/renode_capture.py --init --app <key>` (toolkit README.md:83); `RenodeController.init_pddb()` at renode_lib.py:698. PIN `a` is still the default (CLAUDE.md:104, `--pin` default "a") | FALSE (script gone) |
| A9 | `renode_interact.py full-init 90` | No `full-init` anywhere in the toolkit (grep). Closest: `--init` plus `--boot-wait` (default 60 with --init, else 45; renode_capture.py:431) | FALSE |
| A10 | Toolkit "Screenshot Capture: full boot-to-app workflows" | `renode_capture.py --init --app X [--script capture.py]` does boot, PDDB init, app launch, screenshots; exits non-zero on failed screenshot (README.md:83-86) | TRUE |
| A11 | Toolkit "API Documentation: hard-won discoveries" | `discoveries.md` exists (181 lines) plus `CLAUDE.md` (89 lines), `agents/*.md`, `ecosystem/STANDARDS.md` | TRUE |
| A12 | Toolkit "Developed entirely through LLM-assisted development (Claude Code)... bootstrapping from zero Xous knowledge" | Process claim, no repo evidence beyond CLAUDE.md being an agent guide | UNVERIFIABLE |
| A13 | Timers: "three modes: Pomodoro, Stopwatch, Countdown Collection" | timers README, three screens | TRUE |
| A14 | Timers: "Configurable Pomodoro intervals (25/5/15 min default)" | pomodoro.rs:22 defaults 25/5/15, cycles 4; each configurable 1-180 min | TRUE |
| A15 | Timers: "Stopwatch with lap tracking" | timer.rs:85 `MAX_LAPS = 99` | TRUE |
| A16 | Timers: "Multiple countdown timers" | up to 20 named presets, but **one runs at a time** (README). Blog says "simultaneous countdowns" | CHANGED |
| A17 | Timers: "PDDB persistence for all settings" | settings and presets persist (`timers`/`settings`); running timers do not survive reboot | TRUE (with caveat) |
| A18 | Writer: "Unified writing suite: Markdown Editor, Journal, Typewriter" | writer README | TRUE |
| A19 | Writer: "Markdown editing with preview" | preview exists (F2); renders headings/lists/quotes/code blocks/rules, inline bold and links shown as plain text (README) | TRUE (qualify) |
| A20 | Writer: "Daily journal with timestamps" | One entry per day keyed `YYYY-MM-DD` from the device clock; README does not describe per-entry timestamps | UNVERIFIABLE (say "one entry per day, dated from the device clock") |
| A21 | Writer: "Distraction-free typewriter mode" | forward-only append, draft survives leaving | TRUE |
| A22 | Writer: "**Export to .md and .txt formats**" | No file export. Export is (1) TCP on port 7879 with a one-time code, you redirect to any filename (`nc <ip> 7879 > doc.md`), or (2) USB keyboard autotype. No `.txt`/`.md` writer in code | FALSE |
| A23 | Flashcards: "multi-deck PDDB storage and network import via TCP push" | import port 7878, plus export port 7879, one-time 6-digit code, plaintext (xfer.rs:9-10) | TRUE (add: code, plaintext, export) |
| A24 | Flashcards: "Spaced repetition algorithm" | Leitner boxes 0/1/3/7/16/35 days, Again/Good/Easy; README: "not SM-2... no per-card ease factor". Page is right in spirit; the blog says no SRS | TRUE (but name it Leitner) |
| A25 | Flashcards: "Multiple deck support with progress tracking" | deck list shows due/total; schedule per card; "no statistics beyond the session summary" | TRUE (modest) |
| A26 | Flashcards: "Offline-first, persistent storage" | PDDB decks; network only when you press I/E | TRUE |
| A27 | C64: "Commodore 64 emulator... 6502 CPU, VIC-II, CIA keyboard, monochrome rendering" | all present but limited (below) | TRUE (qualify) |
| A28 | C64: "Full 6502 CPU emulation" | all 151 legal opcodes, decimal mode, stable undocumented opcodes, jam halts; Klaus Dormann test included but ignored by default (README) | TRUE |
| A29 | C64: "VIC-II graphics chip implementation" | text mode and standard hi-res bitmap only; no sprites, multicolour, extended-colour, scroll registers, border effects, raster interrupts (README Limitations). "Most commercial games will not work" | CHANGED (overstated) |
| A30 | C64: "CIA keyboard interface mapping" | CIA1 keyboard matrix plus timers A/B; CIA2 bank select only | TRUE |
| A31 | C64: "Monochrome display adapter **for e-ink**" | Mono conversion with contrast guard is real; the display is a memory LCD, not e-ink | FALSE (e-ink) |
| A32 | Calc: "TI-85 competitor" | No graphing, equation solver, complex numbers, units or programmability (README Limitations). A TI-85 has all of these | FALSE/overstated (UNVERIFIABLE as a comparison; drop it) |
| A33 | Calc: "dual algebraic and RPN modes, full scientific function library, expression parsing" | true; function list in README (functions.rs) | TRUE |
| A34 | Calc: "RPN mode with 4-level stack (X, Y, Z, T)" | rpn.rs: four-level stack with HP stack lift and LASTx | TRUE |
| A35 | Calc: "Trig, log, exp, sqrt, factorial" | plus hyperbolics, cbrt, gamma for non-integer n!, log2, abs, floor, ceil, round | TRUE |
| A36 | Calc: "Memory registers, angle modes, number bases" | M0-M9; DEG/RAD/GRAD; DEC/HEX/OCT/BIN (display only) | TRUE |
| A37 | About Precursor: "**E-ink display**, low power, readable outdoors" | Precursor has a 336x536 1-bit memory LCD (xous-dev-toolkit/agents/testing.md:401 "Memory LCD refresh visible"; agents/encoding.md:101 "memory LCD"; the 01-25 blog itself says "336x536 monochrome LCD, backlit"). No E-ink in any repo doc | FALSE |
| A38 | About Precursor: "FPGA-based, no closed-source CPU microcode" | FPGA SoC is general knowledge; nothing in the repos documents it | UNVERIFIABLE (here) |
| A39 | About Precursor: "Physical keyboard, no touchscreen tracking" | Toolkit assumes keyboard-only input; no touchscreen mentioned | TRUE (consistent) |
| A40 | About Precursor: "Xous OS microkernel" | xous-core microkernel; toolkit docs | TRUE |
| A41 | About Precursor: "Open hardware: schematics and manufacturing files published" | not in repos | UNVERIFIABLE (here) |
| A42 | "PDDB: Plausibly Deniable DataBase for secure, encrypted local storage" | Apps rely on the PDDB; READMEs say "encrypted at rest" and describe basis behaviour (a locked secret basis hides records). Apps add no crypto | TRUE |
| A43 | Page lists no apps beyond 5 | 34 more apps exist (see section 2) | CHANGED (page is stale) |

### 3b. /Users/tyler/colbysdatamovers.com/blog/2026-01-25-precursor-arrives.html

| # | Claim | Current truth (evidence) | Verdict |
|---|---|---|---|
| B1 | "Four apps shipped" / "Apps shipped: 4" / "GitHub repos: 5 (4 apps + toolkit)" | 39 app repos + xous-dev-toolkit now (point-in-time claim for Jan 2026) | CHANGED (stale) |
| B2 | "Lines of Rust ~8,500" (apps only) | Four apps now: writer 3,733 + flashcards 1,720 + c64 4,300 + timers 2,288 = **12,041** lines (all .rs incl. tests). All 39 repos ~71,050. The apps were rebuilt since January | CHANGED |
| B3 | "Screenshots captured: 47" | PNGs in repos today: calc 19, writer 6, flashcards 5, timers 4, toolkit 4; c64 none (removed) = 38 | CHANGED/UNVERIFIABLE (47 not reproducible) |
| B4 | "Development time 28 days (Dec 28, 2025 -> Jan 25, 2026)", "247 commits", "18/14 issues", "1.2 kg beans", "MacBook Pro M2 Max / macOS 15.2 / Renode 1.16.0 / Rust 1.88.0" | No source in repos (precursor-calc has 6 commits now; repos were rebuilt) | UNVERIFIABLE |
| B5 | "LCD display, 336x536 monochrome LCD, backlit" | 336x536 matches toolkit (README:52, STANDARDS.md:122 "Width: 336px", agents/testing.md:401 "Memory LCD"). CLAUDE.md:49 says apps should size from the canvas below the status bar, so usable height is less than 536 | TRUE (blog is right; the apps page is wrong) |
| B6 | "100 MHz RISC-V" | toolkit agents/ideation.md:19 "100MHz CPU" | TRUE |
| B7 | "Zero cloud dependencies / Zero analytics / telemetry / tracking" | no network code beyond LAN transfer; netscan can connect where the user points it | TRUE |
| B8 | Writer: multi-document management (create, open, delete) | also rename (README) | TRUE |
| B9 | Writer: "Line-level syntax styling: Headings -> Large/Bold GlyphStyle, Code -> Monospace, Block quotes indented with bar, Lists preserved prefixes, HR drawn lines" | Now: "Inline markdown is stripped, not styled (one font per line)"; preview renders headings, lists, quotes, fenced code, rules line by line; editor is soft-wrapped monospace | CHANGED |
| B10 | Writer: word/char count in status bar | "Word count in the status line" (char count not mentioned) | TRUE (words) |
| B11 | Writer: "Export via TCP (port 7879) or USB keyboard autotype" | port 7879 (export.rs:8), USB HID autotype | TRUE |
| B12 | Writer: `nc <ip> 7879 > document.md` with no code; "device listens"; "Data leaves only when you explicitly tell it to" | Now requires a **one-time six-digit code sent first** (`echo 123456 \| nc <ip> 7879 > doc.md`); 3 wrong codes or 120 s end it; still plaintext on the LAN | CHANGED |
| B13 | Writer: "**handles documents up to 32KB**" | No such limit in code (grep of writer/core/src and src: no 32 KB/32768/MAX_DOC constant; only name limits MAX_NAME_CHARS=40, SEARCH_LIMIT=50). PDDB values were the old stated reason | FALSE (no evidence; remove) |
| B14 | Writer: "Auto-save on exit and focus loss" | Settings: autosave on leaving or losing focus, default on; F3 saves manually | TRUE |
| B15 | Journal: "One entry per day, automatically keyed by date; auto-creates today's entry on open" | keyed `YYYY-MM-DD`; refuses to open if the clock is unset (0.2 filed entries under 1970-01-01) | TRUE (add clock caveat) |
| B16 | Journal keys: Esc+[ / Esc+] previous/next day, Esc+t today, Esc+/ search | **Removed in v0.3.** The device has no Esc key. Now F1 menu: Previous day, Next day, Today, Search (writer README lines 20, 38, 105; toolkit CLAUDE.md:43, discoveries.md:101-102 "Known bug") | FALSE |
| B17 | Journal: "Search across all entries (substring match)" | case-insensitive, newest first, up to 50 matching lines | TRUE |
| B18 | Journal: "device RTC (llio::LocalTime), no NTP" | README: "from the device clock" | TRUE |
| B19 | Typewriter: forward-only, no backspace/cursor, live word counter, "fresh start each time, session-based" | forward-only yes; but draft is kept when you leave and **resumes next time**; summary shows words, characters, lines | CHANGED (not fresh each time) |
| B20 | Typewriter: "Save to editor documents when done (optional)" | saves as "Freewrite", "Freewrite 2"... | TRUE |
| B21 | "Inspired by iA Writer's Focus Mode and Cold Turkey Writer" | authorial claim | UNVERIFIABLE |
| B22 | "No USB mass storage by default" | not in repos | UNVERIFIABLE |
| B23 | PDDB layout: dict `writer.docs`, keys `md:`, `journal:`, `typewriter:session`; "UTF-8 text, direct storage (no serialization overhead)" | Dicts `writer.docs` (`_index`, `doc_<name>`), `writer.journal` (`_index`, `YYYY-MM-DD`), `writer.settings` (`config`, `typewriter_draft`); **versioned JSON** records | CHANGED |
| B24 | "Encryption: AES-256-GCM-SIV (provided by PDDB layer)" | not documented in repos | UNVERIFIABLE |
| B25 | "Duress password reveals a different set of documents" / multi-basis deniability | PDDB bases exist and READMEs describe basis-aware reloads; "duress password" wording is not in repos, and contacts README says dictionary names and key counts still reveal usage within an unlocked basis | CHANGED (qualify; PDDB behaviour, not app behaviour) |
| B26 | Flashcards: TSV import, flip with Space/Enter, arrows or n/p, deck delete, scrollable list | all present (flashcards README keys table) | TRUE |
| B27 | Flashcards: "Up to 500 cards per deck" | deck.rs:11 `MAX_CARDS = 500` | TRUE |
| B28 | Flashcards: "Built-in demo deck on first launch" | "Xous Basics", added once | TRUE |
| B29 | Flashcards: "Duplicate deck name handling" | not stated in current README | UNVERIFIABLE |
| B30 | Flashcards: "press 'i'... Device shows 'Listening on port 7878'... `cat spanish.tsv \| nc <ip> 7878`" | Port 7878 yes (xfer.rs:9). **Now requires a one-time six-digit code**: `(echo 123456; cat deck.tsv) \| nc <ip> 7878`; device shows the code and the command; 3 wrong codes or 120 s end it; plaintext | CHANGED |
| B31 | TSV format `#name:Spanish Vocab` then `front\tback` | README same (optional `#name:`, `#` comments, blank/one-sided lines skipped) | TRUE |
| B32 | Flashcards storage: dict `flashcards`, `deck_index`, `deck_0..deck_N` binary-serialized, `DeckMeta`, "max deck size 500 (fits in 64KB PDDB value limit)" | Now one key per deck `deck.<16 hex>` as versioned JSON plus `meta`; 0.1 `_index`/`deck.<name>` binary converted on first start | CHANGED |
| B33 | "Import protocol: TCP on port 7878, one connection per deck" | 64 KB cap, 2 min, code | TRUE (port) |
| B34 | "**Future: Spaced Repetition** - current version: basic flashcards, no SRS" | v0.2 has Leitner SRS (intervals 0/1/3/7/16/35 days). SM-2 not implemented | CHANGED (shipped, as Leitner) |
| B35 | "Parsing JSON on a 100MHz CPU is noticeable" (why TSV) | rationale | UNVERIFIABLE (apps now use JSON for storage) |
| B36 | C64: "All 151 legal opcodes, BCD arithmetic, IRQ/NMI/RESET" | 151 legal opcodes and decimal mode yes; README lists "level-triggered IRQ", and **no CIA2 NMI**; jam opcodes halt | TRUE (NMI/RESET not stated now) |
| B37 | C64: "Illegal opcodes treated as NOP" | now: stable undocumented opcodes implemented; jam opcodes halt the CPU and the UI says so | CHANGED |
| B38 | C64 video: text 40x25, hi-res bitmap 320x200, bank switching for char ROM, monochrome luminance threshold | vic.rs WIDTH 320 HEIGHT 200; text and standard hi-res bitmap; README: "contrast guard" | TRUE |
| B39 | C64 memory: BASIC ROM optional 8 KB at $A000, KERNAL 8 KB at $E000, char ROM 4 KB, "minimal built-in char ROM" | memory.rs:9-11 8192/8192/4096; built-in original stand-in font and mini KERNAL (rom_stub.rs); ROM images must be exactly those sizes | TRUE |
| B40 | C64 input: CIA1 8x8 matrix; RUN/STOP -> Esc, RETURN -> Enter | **Esc does not exist on the device.** Now backtick = RUN/STOP, Enter = RETURN, Backspace = INST/DEL, F1/F2/F3 = C64 F1/F3/F5, **F4 exits** | CHANGED/FALSE (Esc) |
| B41 | C64: `(echo "gamename"; cat game.prg) \| nc <ip> 6464` | Port 6464 yes. Name line must be `A-Za-z0-9 ._-` 1-32 chars, e.g. `printf 'mygame.prg\n' \| cat - mygame.prg \| nc <ip> 6464`; ROMs via `rom:basic|kernal|chargen`; listens only on the Import screen; unauthenticated; 5 s read / 30 s total / 64 KB | TRUE (port), CHANGED (format details) |
| B42 | C64 "What's Not Implemented": SID audio, sprites, raster interrupts, illegal opcodes, smooth scrolling, multicolour, disk | Still no SID, sprites, raster IRQ, multicolour, disk, tape, joystick, TOD. "CIA timer IRQ works" is true | TRUE (mostly) |
| B43 | "SID audio: Precursor has piezo speaker only" | toolkit ideation.md:21 says "No audio speaker | Visual/vibration feedback only (no audio alerts)"; timers README: "No sound". Not supported by repos | FALSE/UNVERIFIABLE |
| B44 | "**50,000 C64 cycles per frame (PAL ~50 fps -> ~20 fps emulated)**" | `CYCLES_PER_FRAME = 50_000` (machine.rs:11) per 50 ms tick = 20 fps nominal; README: "Timing is approximate (about 50,000 cycles per 50 ms frame). Speed on the device is untested and may be slower than a real C64" | TRUE (constant), UNVERIFIABLE (actual fps) |
| B45 | "Each instruction costs ~100 CPU cycles on 100 MHz" | no measurement in repos | UNVERIFIABLE |
| B46 | C64 "No dependency on copyrighted ROMs... without ROMs still runs .PRG files that don't need BASIC" | README: no ROMs shipped; stub has "no BASIC, no keyboard handling"; plain BASIC programs refused, machine-code PRGs run | TRUE |
| B47 | "Target game library: Zork I-III, Elite, Ultima, Archon, Boulder Dash, Rogue, Tetris clones..." | README: "Most commercial games will not work" (no sprites, no raster IRQ, no SID); Elite/Archon/Boulder Dash need sprites or raster effects. None tested | FALSE/overstated |
| B48 | "Storage: .PRG files stored encrypted" | stored in PDDB (`c64.games`), encrypted at rest by PDDB | TRUE |
| B49 | Maelee's games "Hexmaze, Starport, Tower of Glyphs" | roadmap statement | UNVERIFIABLE |
| B50 | Timers: "(GitHub repo in progress)" / "Coming soon" | repo exists (precursor-timers v0.3.0) | CHANGED |
| B51 | Timers: Pomodoro 25/5/15, long break every 4th, configurable | pomodoro.rs:22 | TRUE |
| B52 | Timers: "**Audio alert on completion (piezo beep)**" | **No sound** (README: "No sound (0.2's audio toggle did nothing and was removed)"). Alerts are vibration and/or notification | FALSE |
| B53 | Timers: "PDDB persistence for interval settings", "session counter", "auto-start next interval (optional)" | persistence yes; session count yes; next phase "starts by itself with an alert" (not optional in README) | TRUE / CHANGED (auto-start not documented as optional) |
| B54 | Stopwatch: "up to 50 laps" | MAX_LAPS = 99 (timer.rs:85) | CHANGED |
| B55 | Stopwatch: "Millisecond precision (RTC-backed)" | README: total in tenths, timestamps from ticktimer | CHANGED (tenths of a second) |
| B56 | Stopwatch: "Export lap times via TCP or USB autotype" | No export in timers README or code (no net code in precursor-timers) | FALSE |
| B57 | Countdown: "Quick presets (5m, 10m, 15m, 30m, 60m)" | not present; presets are user-created named countdowns | FALSE |
| B58 | Countdown: "Custom durations up to 24 hours" | MAX_DURATION_MS = 24 h (countdown.rs:10) | TRUE |
| B59 | Countdown: "Simultaneous countdowns" | "One runs at a time; starting another asks first" | FALSE |
| B60 | Countdown: "Persistent across app restarts" | presets persist; "Running timers are not saved: a reboot stops them" | CHANGED |
| B61 | Toolkit: `scripts/renode_interact.py screenshot / init-pddb a / launch-app Flashcards 1 / full-init 90` | File and all four subcommands gone (see A7-A9). `launch-app` -> `RenodeController.launch_app(index)` (renode_lib.py:745) / `--app <manifest key>` in renode_capture.py | FALSE |
| B62 | Keyboard hold: "500ms hold threshold"; fix is to send Press and Release in one write | 500 ms threshold TRUE (CLAUDE.md:101). Fix **changed**: `timed_key()` pauses emulation, presses, runs `hold_ms` (1-499) of emulated time, releases, resumes (scripts/README.md:83; renode_lib.py:477-519). Text via `InjectLine` still recommended | CHANGED |
| B63 | PDDB init: PIN 'a'; steps Down,Down,Enter; "~6 minutes" | PIN `a` TRUE (CLAUDE.md:104). `init_pddb` "waits at least 180 s, then until the screen holds still for 60 s (900 s cap)" (scripts/README.md:109) | CHANGED (timing) |
| B64 | Screenshots via `sysbus.memlcd ReadAllLines` -> base64 -> PIL | Real command: `sysbus.memlcd TakeScreenshot`, PNG extracted and validated in pure Python (renode_lib.py:~610, `extract_png`, `decode_png_rows`) | CHANGED |
| B65 | "Every README screenshot created by automation" | Only calc (19), writer (6), flashcards (5), timers (4) have screenshots; all are from **old versions** (calc 0.1, others 0.1/0.2) and the READMEs say they show old layouts; c64's were deleted (showed Shellchat) | CHANGED |
| B66 | Toolkit includes `CLAUDE.md` and `discoveries.md` | both at toolkit root | TRUE |
| B67 | CLAUDE.md "covers... complete app template with boilerplate" | template lives in `templates/app/` and `bin/new-app` scaffolds it; CLAUDE.md (89 lines) is deliberately short and points to `agents/*.md` | CHANGED |
| B68 | discoveries: "Do NOT depend on ux-api or blitstr2... use gam::menu::*" | not repeated in the current docs I grepped; STANDARDS.md says to use `gam` and `precursor_appkit` | UNVERIFIABLE |
| B69 | discoveries: "Point coordinates are isize" | STANDARDS.md:376 "All drawing coordinates use isize"; discoveries.md:148 | TRUE |
| B70 | discoveries: "std::net works directly (TcpListener, TcpStream, UdpSocket)" | STANDARDS.md:333 "Use std::net directly"; c64/flashcards/gitlog/writer use TcpListener | TRUE |
| B71 | discoveries: "GAM rate limits redraws to ~30fps (33ms)" | not found in current docs | UNVERIFIABLE |
| B72 | discoveries: "APP_NAME must match context_name in apps/manifest.json" | Now: use generated `gam::APP_NAME_<KEY>` constant, whose value is the manifest `context_name` (CLAUDE.md:31, STANDARDS.md:80-83). Hand-typed strings make `register_ux` return Ok(None) | CHANGED |
| B73 | discoveries: "Menu key is '∴' for BOTH opening AND selecting" | `∴` "never reaches an app" (README.md:50, STANDARDS.md:45: never bind Menu/Back/Quit to it) | FALSE (now) |
| B74 | discoveries: "Arrow keys Unicode, NOT Apple PUA" | arrows U+2190-2193 (README.md:50) | TRUE |
| B75 | discoveries: "main() -> ! with xous::terminate_process(0) mandatory" | Now apps leave with `gam.relinquish_focus()` and stay running; `terminate_process` is listed as an anti-pattern in app READMEs ("`terminate_process`... replaced by relinquish_focus") | CHANGED |
| B76 | discoveries: "Apps must handle FocusChange" | CLAUDE.md:50, STANDARDS.md:373 | TRUE |
| B77 | **Keyboard standard "F1: Menu / Help; F4: Exit/Back; F2/F3 app-specific. Writer F2 = Toggle Preview, F3 = Save; Timers F2 = Start/Stop, F3 = Reset; Flashcards F2 = Flip, F3 = Next"** | Toolkit STANDARDS.md:40-42: F1 App menu/Help, F2/F3 app-specific, **F4 = "Settings / Mode toggle, or Back"**, plus Backspace = back and `q` = leave. Writer F2 preview, F3 save TRUE; Timers F2 start/pause, F3/`r` reset TRUE; Flashcards F2 flip, F3 next TRUE | TRUE (F4 wording: "Back/Exit" in apps) |
| B78 | "F1 first item always Help"; "F4 confirms before data loss" | writer: F4 with unsaved changes asks y/n/cancel; other apps vary. F1 first-item-is-Help not verified for every app | UNVERIFIABLE (partly TRUE) |
| B79 | "This standard has been adopted across all apps. Writer, Timers, Flashcards, C64 implement F1/F4" | All 39 current apps exit on Backspace or F4 (READMEs). C64: F1/F2/F3 map to C64 F1/F3/F5 while running; F4 exits | TRUE (broader than claimed) |
| B80 | LoRa: RAK3172 module, NextPCB Accelerator #5, timeline Feb-Jun 2026, Meshtastic | no LoRa code in any repo; roadmap, dates in the past | UNVERIFIABLE (and stale) |
| B81 | LoRa interface: "8-pin connector to iCE40 UP5K FPGA GPIOs" | HW Tools README: "8 battery-compartment GPIO pins; only pins 2 and 5 can be read (ADC)... cannot set output level". Feasibility of UART via those pins is not covered | UNVERIFIABLE |
| B82 | "Precursor ... WiFi has an IP stack" (why LoRa) / apps are zero network | Precursor has a WF200 WiFi module (toolkit agents/networking.md:19-20); apps use it for LAN transfer | TRUE |
| B83 | "The Writer app written today will run on Precursor hardware in 2036" | forward-looking; and no app has run on hardware per READMEs | UNVERIFIABLE (do not repeat) |
| B84 | Title/body: "Four production applications" / "Four weeks later: four production apps" / "Build. Ship. Done." | Every README says Renode capture not run; none verified on hardware; toolkit gate is build+clippy+host tests+lint | CHANGED (overstated; say "hosted-tested", not "production") |

### 3c. /Users/tyler/colbysdatamovers.com/blog/2026-02-04-precursor-calc-journey.html

| # | Claim | Current truth (evidence) | Verdict |
|---|---|---|---|
| C1 | "TI-85 competitor" | see A32 | FALSE/overstated |
| C2 | "Shunting-yard algorithm" / "Dijkstra's... two stacks" | precursor-calc/core/src/parser.rs is "recursive-descent infix parser" (calc README Layout). `grep -i shunting` in core/src: no hits | FALSE (now) |
| C3 | `2+3*4 = 14` | calc precedence; core/src/tests.rs key-sequence tests | TRUE |
| C4 | "unary minus tricky; tokenizer tracks whether previous token expects an operand" | README now: unary minus binds looser than `^` (`-2^2 = -4`) and tighter than `*`; leading `-` is negation, other leading operators continue `ans` | CHANGED (details differ) |
| C5 | `sin(90) = 1` in DEG, `sin(pi/2)` in RAD | "Exact trig at quarter turns in DEG/GRAD (`sin(180) = 0`)". RAD path uses float, so `sin(pi/2)` is 1 to display precision (12 sig. digits) | TRUE (DEG); TRUE-ish (RAD) |
| C6 | "Type function names directly: sqrt(2) = 1.41421356..." | typed names work; display up to 12 significant digits (format.rs) | TRUE |
| C7 | "history tape shows previous calculations" | algebraic results only; last 20 saved; Up/Down recall into input line | TRUE (qualify) |
| C8 | "(2+3)*4 = 20, arbitrary nesting, no limits" | works; "no limits" not verified | TRUE / UNVERIFIABLE (limits) |
| C9 | "Press **M** to toggle modes" | README: `M` toggles ALG/RPN | TRUE |
| C10 | "Four registers: T, Z, Y, X; LastX recall" | X, Y, Z, T and LASTx; `l` recalls LASTx in RPN | TRUE |
| C11 | RPN example: 2 Enter 3 + = 5 | ENTER copies X to Y; `1 ENTER 2 ENTER 3 + +` is 6 (README) | TRUE |
| C12 | "Press **A** to cycle DEG -> RAD -> GRAD" | functions.rs:21-23 | TRUE |
| C13 | "Ten memory registers. **S** then digit to store, **K** then digit to recall. Persistent, encrypted in PDDB" | M0-M9; `S`+0-9 stores ans (ALG) or X (RPN); `K`+0-9 recalls; saved in `calc.settings`/`state` JSON | TRUE |
| C14 | "Press **B** to cycle DEC -> HEX -> OCT -> BIN" | functions.rs:74-77; display-only, entry always decimal; `~` marks truncated fraction; beyond 64-bit shows "out of range" | TRUE (add display-only) |
| C15 | "~3,500 lines of Rust across 11 modules" | 3,030 lines in 11 .rs files including tests and shell; ~2,400 without tests.rs | CHANGED (about 3,000) |
| C16 | Module list `algebraic.rs, rpn.rs, functions.rs, display.rs, keymap.rs, memory.rs, storage.rs, ui.rs, app.rs, main.rs` | Current files: core/{calculator, format, functions, lib, menu, parser, rpn, tests}.rs and src/{main, app, ui}.rs. No `algebraic.rs`, `display.rs`, `keymap.rs`, `memory.rs`, `storage.rs` | FALSE (now) |
| C17 | "uppercase letters for commands (M mode, A angle), lowercase for expression input" | calc README: "There is no Shift layer: upper-case letters are commands, lower-case letters are typed into expressions" | TRUE |
| C18 | "Adaptive display: small numbers decimals, large scientific, trailing zeros trimmed" | README: up to 12 significant digits, scientific from 1e12 and below 1e-4 | TRUE |
| C19 | "Apache 2.0 license" | Cargo.toml license "Apache-2.0"; LICENSE file | TRUE |
| C20 | "All 3,500 lines. **All 19 screenshots**" | 19 PNGs exist in precursor-calc/screenshots, but README: "from version 0.1 and show the old layout" | TRUE (count) / CHANGED (stale) |
| C21 | "github.com/tbcolby/precursor-calc" | origin remote https://github.com/tbcolby/precursor-calc.git | TRUE |
| C22 | "Stores your settings encrypted in the PDDB" | `calc.settings`/`state`; PDDB encryption only | TRUE |
| C23 | "Calculator at rest. No notifications" / "Every transistor documented, every line of firmware open source, no proprietary black boxes" | hardware claims, not in repos | UNVERIFIABLE |
| C24 | "Writer, Flashcards, C64 Emulator, Timers, and now the Calculator" (5 apps) | 39 apps now | CHANGED (stale) |
| C25 | "Shunting-yard is from 1961... 65 years ago" | factual aside (Dijkstra, 1961) | TRUE (but the calculator no longer uses it) |
| C26 | calc not yet verified on Renode or hardware | calc README says so; the post implies working screenshots | CHANGED |
| C27 | "336x536 monochrome display" | consistent with toolkit | TRUE |

## 4. Toolkit command and file check (/Users/tyler/Dev/precursor/xous-dev-toolkit)

Exists today: `README.md`, `CLAUDE.md`, `discoveries.md`, `XOUS_CORE_ANCHOR`, `bin/{new-app,integrate,lint,status}`, `appkit/` (`precursor-appkit`), `precursor-appkit-core/`, `scripts/{renode_lib.py,renode_capture.py,usb_log_monitor.py,README.md}`, `agents/*.md` (architecture, build, encoding, graphics, ideation, networking, randomness, review, storage, supervisor, system, testing), `ecosystem/{STANDARDS,PIPELINE,REMEDIATION,FLYWHEEL,LESSONS,TRACKER,DOCS_AUDIT}.md`, `templates/app/`, `tests/`, `screenshots/` (4 PNGs), `notion_update.md`.

Does NOT exist (referenced by the site): `scripts/renode_interact.py` and its subcommands `screenshot`, `init-pddb`, `launch-app`, `full-init`. Git history: deleted in commit 193dbdc "Overhaul agent system and consolidate Python automation" and 1cf105c "Make Renode automation fail loudly and own its process" (earlier: 76135d2 initial). Current equivalent commands (toolkit README.md:83-84):

```
python3 scripts/renode_capture.py --init --app tally            # blank flash, format PDDB with PIN 'a'
python3 scripts/renode_capture.py --app tally --script ../precursor-tally/capture.py
```
Flags: `--xous-root`, `--init`, `--pin` (default a), `--app` (manifest key), `--app-index`, `--screenshots`, `--script`, `--port` (default 4567 or $RENODE_PORT), `--boot-wait`, `--renode-log`, `-q` (renode_capture.py:416-433).

Other toolkit commands now available: `bin/new-app <name> "<Display>"`, `bin/integrate --check|--grade <repo>`, `bin/lint`, `bin/status`; hosted run `cargo xtask run <app>`; Renode image `cargo xtask renode-image <app>`; device `cargo xtask app-image <app>`.

Toolkit key facts that contradict old pages: no Esc key on the device or in Renode (CLAUDE.md:43); F1-F4 = 0x11-0x14; `∴` never reaches apps; `UxType::Framebuffer` for apps (not Chat, which reserves an IME line); build is pinned to an xous-core anchor commit (`XOUS_CORE_ANCHOR`, STANDARDS.md:302 "31fbe23").

## 5. Hardware claims: wrong or overstated

| Claim (page) | Problem | Source |
|---|---|---|
| "E-ink display, low power, readable outdoors" (precursor-apps.html) and "Monochrome display adapter for e-ink" (C64 bullet) | Wrong. Precursor has a 336x536 1-bit **memory LCD** (backlit per the blog). Memory LCDs are low-power and sunlight-readable, so the benefit can be kept without the word "E-ink" | xous-dev-toolkit/agents/testing.md:401 ("Memory LCD refresh visible"); agents/encoding.md:101 ("memory LCD"); the 2026-01-25 blog itself says "336x536 monochrome LCD" |
| "Precursor has piezo speaker only" / "Audio alert (piezo beep)" (blog 01-25) | Not supported. Toolkit says no audio speaker, use visual/vibration feedback; timers README says no sound; alerts are vibration/notification | xous-dev-toolkit/agents/ideation.md:21; precursor-timers/README.md Limitations |
| "336x536" as the app drawing area | Canvas sits below the status bar; apps must size from `gam.get_canvas_bounds`, never assume 336x536 (several READMEs remove hard-coded 336x536 as a fixed bug) | xous-dev-toolkit/CLAUDE.md:49 |
| "Menu key is ∴, F1/F4..." and any Esc-based commands | The device has no Esc key; ∴ opens the system menu and the app never receives it | CLAUDE.md:43; STANDARDS.md:29,45 |
| "100 MHz RISC-V" | Consistent with toolkit (ideation.md:19) | OK |
| "Export... because Precursor doesn't expose USB mass storage by default" | not in repos | unverifiable |
| "iCE40 UP5K FPGA GPIOs" for the LoRa module, "FPGA-based, no closed-source CPU microcode", "open schematics" | Not documented in repos or toolkit. (Orchestrator note, general knowledge, verify against betrusted.io before publishing: the SoC FPGA is a Xilinx Spartan-7 XC7S50 running an open VexRiscv RV32 design; the iCE40 UP5K is the embedded controller. The FPGA silicon itself is proprietary, so "every transistor documented" overclaims; the defensible claim is that the CPU design is open RTL you can inspect and rebuild.) | none in repos |
| "Zero-trust security model" (blog stats) | marketing term, no repo definition | none |
| "All apps ... Security: encrypted by PDDB" | PDDB encrypts at rest, nothing more; READMEs repeatedly say anyone who can unlock the basis can read the data, delete is not secure erasure, and LAN transfers are plaintext with only a 6-digit anti-injection code | app READMEs (contacts, keygen, otp, flashcards, writer) |
| "Production applications"/"Four production apps" | none has run in Renode or on hardware as of the READMEs; gate is host build/clippy/tests/lint | every README; toolkit CLAUDE.md:11 |

## 6. Counts of verdicts (first-listed verdict in each row, machine-tallied)

| page | rows | FALSE | CHANGED | UNVERIFIABLE | TRUE |
|---|---|---|---|---|---|
| precursor-apps.html (A1-A43) | 43 | 8 | 5 | 4 | 26 |
| 2026-01-25 blog (B1-B84) | 84 | 10 | 25 | 14 | 35 |
| 2026-02-04 blog (C1-C27) | 27 | 3 | 4 | 1 | 19 |
| total | 154 | 21 | 34 | 19 | 80 |

Rows with a compound verdict such as "TRUE (qualify)" or "CHANGED/FALSE" are counted under the first word.
