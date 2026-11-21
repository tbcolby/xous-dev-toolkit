# colbysdatamovers.com: Precursor update notes

> **Status 2026-11-21: applied and live** (site commits 415a63b, 9f2c863). `precursor-apps.html` rewritten (40 apps); new post `blog/2026-11-21-precursor-40-of-40.html`; launch and calc posts annotated with update boxes and inline corrections; sitemap lastmod bumped; unescaped `<...>` in code blocks fixed on the launch post and three Salesforce posts.
>
> **Correction to §1/§3.4 below:** the Precursor posts are *deliberately* `noindex` and absent from `blog/index.html` and `sitemap.xml`. Tyler parked the Precursor and commerce blog cluster on 2026-07-14 (site commit bca53b3) as off-identity for the Salesforce practice. That is not a bug. The new post follows the same parking and is linked only from the showcase page. To un-park any post, delete its `<meta name="robots" content="noindex, nofollow">` line and add it to the index and sitemap.
>
> **Defaults used for §6** (revisit any time): display names as the apps ship them; LAN transfers disclosed as plaintext with a one-time code (and c64's listener as code-free); Game Boy presented as a CPU debugger; no specific FPGA part names; the January roadmap marked "as written, not updated". Still open: Renode screenshots (then replace the calc post's v0.1 images), and the LoRa / C64-games roadmap status.

Written 2026-11-21, after all 40 Precursor apps reached the toolkit's 10/10 gate. This file is the brief for updating every Precursor page and blog post on colbysdatamovers.com. Nothing on the site has been changed yet.

**Sources (all in this repo, all current):**
- `ecosystem/website/SITE_FACTS.md` holds verified per-app facts (features, limits, network use, test counts) and a 154-row claim check of the three existing pages, with each row tagged TRUE, FALSE, CHANGED or UNVERIFIABLE and backed by file:line evidence. Claim IDs below (A12, B40, C7) refer to its section 3.
- `ecosystem/audits/2026-11-20/` is the baseline audit (0/40 apps built; mean score 3.5/10).
- `ecosystem/REMEDIATION.md`, `LESSONS.md` and `FLYWHEEL.md` hold the method, per-round log and lessons.
- The site repo is `~/colbysdatamovers.com` (GitHub `tbcolby/colbysdatamovers.com`, deployed by Cloudflare Pages from `main`). Ignore the stale second clone at `~/tbcolby/colbysdatamovers.com` (last commit January 2026).

---

## 1. Where Precursor appears on the site

| File | Role | State |
|---|---|---|
| `precursor-apps.html` | Labs showcase page | Badly stale: lists 5 apps, says "6 Apps Published", "E-ink display", dead toolkit commands. **Rewrite.** |
| `blog/2026-01-25-precursor-arrives.html` | Launch post | Historical, with 10 FALSE and 25 CHANGED claims. **Keep as history and add a dated update block, plus inline corrections where it is wrong rather than merely dated.** |
| `blog/2026-02-04-precursor-calc-journey.html` | Calculator post | 3 FALSE, 4 CHANGED; 19 screenshots from v0.1 in `blog/img/calc/`. **Update block + corrections; replace screenshots after a Renode capture.** |
| `blog/index.html` | Blog index | **Neither Precursor post is listed.** Add both, plus any new posts. |
| `sitemap.xml` | Sitemap | Has `precursor-apps.html` (lastmod 2026-07-14), **but neither post.** Add them; bump lastmod on edit. |
| Footer (every page) | "Resources → Precursor Apps" link | Fine as is. |
| `llms.txt` | LLM site summary | No Precursor mention. Optional: one line under a Labs heading. |
| `CONTENT_SPEC.md`, `COPY_AUDIT.md`, `SITE_OVERHAUL_PLAN.md` | Internal | COPY_AUDIT scores the launch post 19 "tells" and the calc post 12. Clear them during the rewrite. |

**House rules from the site repo:**
- No em dashes in shipping HTML. "Gate 1" in COPY_AUDIT enforces this.
- Avoid the COPY_AUDIT tells (filler, rhetorical questions, triads, hyphen-glue).
- Keep the "labs project, separate from our Salesforce data practice" disclaimer.
- Commit format is `[Content] ...`, `[Fix] ...` (CONTENT_SPEC §1018).

---

## 2. Corrections to apply everywhere

1. **Display:** a 336x536 1-bit **memory LCD**, not e-ink. Its real benefits are low power and readability in sunlight. Apps draw below the status bar and must not assume the full 336x536 (A-rows on the apps page; the C64 "e-ink adapter" bullet).
2. **No audio:** Precursor has no speaker usable by apps. Alerts are vibration or notification. Remove "piezo beep" (timers) and "piezo speaker" (C64 SID note).
3. **No Esc key.** `∴` opens the system menu and never reaches apps. Apps use F1–F4 menus. Remove the Esc-based journal shortcuts from the Writer copy.
4. **"Production" means nothing has run in Renode or on hardware yet.** The 10/10 gate is a hosted build, strict clippy, host tests and the toolkit lint. Say "built and tested on the host; device testing in progress", not "production-ready".
5. **"100% offline / zero cloud":** no app touches the internet or any cloud. True, keep it. But five apps use the **local network**:
   - c64 listens on :6464 with no authentication.
   - flashcards (:7878 import, :7879 export), writer (:7879 export) and gitlog (:7880) use a 6-digit one-time code, with **plaintext** transfer.
   - Net Check makes outbound TCP connects to a host the user types.

   Say "offline by default; optional local-network transfer" and disclose plaintext.
6. **PDDB:** encrypts at rest, with plausible deniability through *secret bases*. The launch post's "duress password reveals a different set of documents" overstates it; describe secret bases accurately. Apps add no crypto of their own, and delete is not secure erasure.
7. **Hardware provenance:** cut "every transistor documented", "the only truly auditable mobile platform" and "first ... designed from silicon up". The FPGA is proprietary silicon. The defensible claim is that the CPU is open RTL (VexRiscv) loaded onto an FPGA, so you can inspect and rebuild it, and the FPGA resists silicon-level implants. Verify the exact parts (Spartan-7 SoC, iCE40 UP5K EC) at betrusted.io before naming them.
8. **Toolkit commands:** `scripts/renode_interact.py` (`screenshot`, `init-pddb`, `full-init 90`) no longer exists. Current workflow:
   ```
   bin/new-app tally "Tally"                      # scaffold from the template
   bin/integrate --grade ../precursor-tally       # hosted build + clippy + tests + lint (the 10/10 gate)
   python3 scripts/renode_capture.py --init --app tally
   python3 scripts/renode_capture.py --app tally --script ../precursor-tally/capture.py
   ```
   `CLAUDE.md` and `discoveries.md` still exist under those names.
9. **Counts:**
   - 40 apps: 39 `precursor-*` repos plus `carse-automata`. `precursor-apps` is the retired scaffold; don't list it.
   - About 1,400 host tests (1,404 counted).
   - About 71,000 lines of Rust.
   - All Apache-2.0.

---

## 3. Page-by-page plan

### 3.1 `precursor-apps.html`: rewrite

Proposed structure:
1. **Hero:** "40 open-source apps for Precursor", with the labs disclaimer. One line on the platform: a pocket device with a 1-bit memory LCD, physical keyboard, an open RISC-V CPU on an FPGA, and the Xous microkernel.
2. **Status strip (replaces "6 / 100% / 0 / Rust"):**
   - 40 apps
   - ~1,400 host tests
   - 0 cloud dependencies
   - Rust, Apache-2.0
   - An honest note: "Built and tested on the host. Device screenshots coming."
3. **Toolkit feature** (keep it first, since the toolkit is the real product). Update to the current toolkit:
   - **The 10/10 gate:** `bin/integrate --grade`.
   - **App template:** `bin/new-app`, which gives a pure `core/` crate with host tests plus a thin Xous shell.
   - **Shared libraries:** `precursor-appkit` and `precursor-appkit-core` provide:
     - TRNG-backed randomness without modulo bias
     - PDDB record storage that never overwrites unreadable data
     - a suspend-safe clock
     - fixed-point trigonometry and line clipping
     - text fields and wrapping
     - batched drawing
   - **`bin/lint`:** 14 rules that catch the API mistakes found in the audit.
   - **Renode capture:** `scripts/renode_capture.py`.
   - **CI:** pinned to an xous-core anchor commit and Rust 1.88.0.

   Replace the code sample with the commands in §2.8.
4. **App catalog by category (all 40)**, from the table in §5: name, one line, repo link, and a "LAN" badge where relevant. Six groups: Productivity (9), Security (3), Developer tools (9), Games (7 incl. Carse Automata), Emulators (2), Generative art (10).
5. **"How these were built"** (short, link to the new post in §4.1): an audit that found 0 of 40 building, a shared toolkit and gate, then rebuilt app by app by AI workers, with every result re-checked independently.
6. **About Precursor:** keep it, with corrections 1, 2 and 7 applied. Keep the Crowd Supply and xous-core links.
7. **Meta:** update `<title>`/description (it currently names only timers, writer, flashcards, C64), add `og:` tags if the site template uses them, and bump the sitemap lastmod.

Per-card copy notes (from SITE_FACTS §1):
- **C64:** "hobby-grade emulator for text-mode software and BASIC". Drop "full VIC-II" and the Elite/Archon target list; there are no sprites, multicolour or raster IRQs.
- **Game Boy:** this is a **CPU step debugger**, not an emulator. Name the card accordingly, or omit the Emulators group and put it under Developer tools.
- **Writer:** export goes over the LAN (one-time code) or USB autotype. There is **no** .md/.txt file export and no 32 KB limit claim.
- **Flashcards:** **now has spaced repetition** (Leitner boxes at 0/1/3/7/16/35 days, not SM-2). TSV decks are pushed over the LAN with a 6-digit code.
- **Timers:**
  - Pomodoro, a stopwatch (up to 99 laps) and named countdowns.
  - All three keep running after you leave the app.
  - Alerts vibrate or post a notification.
  - No presets, and only one countdown runs at a time.
- **Calculator:**
  - Algebraic (a recursive-descent parser, not shunting-yard) and HP-style 4-level RPN.
  - Drop "TI-85 competitor": it has no graphing, solver or complex numbers.
- **Renamed apps:** use the display names Cron Explainer, Net Check, IFS Fractals, Text Adventure, RSS Viewer and SQL Explorer, and keep the "does not ..." honesty from each README (RSS Viewer does not fetch; SQL Explorer is not SQLite; the text adventure is not Zork). This is **pending Tyler's sign-off on the renames (§6).**

### 3.2 `blog/2026-01-25-precursor-arrives.html`: annotate, don't rewrite

Keep the post as a dated record. Add a boxed **"Update, November 2026"** block under the byline:
> Since this post: all four apps were rebuilt, and the studio grew to 40 open-source apps, all passing the toolkit's quality gate (hosted build, strict lint, host tests). Some details below have changed; corrections are marked inline. Current list: /precursor-apps.html. How the rebuild worked: [new post].

**Inline corrections** (mark them, e.g. `<ins>`/"Correction:"). These are the FALSE rows in SITE_FACTS §3b:
- **Hardware:** the duress-password claim (correction 6), piezo audio (2), and "every transistor"/"only truly auditable"/"first" (7).
- **Toolkit:** every `renode_interact.py` command and the PDDB-init steps (8). Keep the keyboard-hold-timing story, which is still a real lesson, but point to `renode_capture.py`.
- **Writer:** the Esc-based journal keys, the .md/.txt export, and the 32 KB limit.
- **Flashcards:** "no SRS". It now has Leitner spaced repetition.
- **Timers:**
  - "coming soon / repo in progress": it is published.
  - "piezo beep"
  - "50 laps" should be 99
  - the countdown presets
- **C64:** the target game list, VIC-II scope, and "50K cycles per frame" as stated (check the current value in SITE_FACTS B-rows).
- **"Four production applications":** see correction 4.
- **Roadmap section** (LoRa February–June 2026, Maelee's C64 games June 2026): **status unknown to this session; Tyler to supply** (§6). A `PrecursorLoRa` repo exists. Either report progress or mark the roadmap as superseded.
- **Clear the 19 COPY_AUDIT tells** while editing. Most are filler and triads in the "Why This Matters" and "What We Learned" sections.

### 3.3 `blog/2026-02-04-precursor-calc-journey.html`: annotate

- Add an update block. The calculator was rebuilt (70 host tests); the parser is now recursive-descent.
- Fix C-rows:
  - "shunting-yard" is now recursive-descent. Keep the shunting-yard history as "the first version used...".
  - "~3,500 lines / 11 modules" is now about 3,030 lines; use the current module list.
  - Verify the key bindings M/A/B/S/K against current calc (SITE_FACTS C-rows give current values).
  - "TI-85 competitor" becomes "TI-style scientific calculator with RPN".
- **Screenshots:** all 19 images in `blog/img/calc/` show the v0.1 layout. Either caption them "v0.1 (January 2026)" now, or replace them after a passing Renode capture (`precursor-calc/capture.py`). Don't publish screenshots that don't come from a passing capture run.
- Clear the 12 COPY_AUDIT tells.

### 3.4 `blog/index.html` and `sitemap.xml`

- Add cards for both existing Precursor posts, plus the new ones in §4, in date order.
- Add every Precursor post URL to `sitemap.xml`, and bump `precursor-apps.html` lastmod.

---

## 4. New posts to write

### 4.1 "From 0 of 40 to 40 of 40" (the rebuild story; highest value)

Working title: *Rebuilding a 40-app studio for Precursor: an audit, a gate, and a flywheel*.

Facts to use (all verifiable in this repo):
- **Baseline (2026-11-20 audit):**
  - 0 of 40 apps built against current xous-core; mean score 3.5/10.
  - The root causes were shared, not per-app:
    - wrong arrow-key codes (Apple private-use U+F700 instead of U+2190–2193)
    - a Menu key bound to `∴`, which the system swallows
    - a PDDB API that doesn't exist (`set_len`)
    - a GAM API invented in about 30 apps
    - storage rewritten on every background event
  - Bad guidance had copied the same mistakes into every app.
- **The fix was tooling first:**
  - a pinned xous-core anchor and Rust version
  - `bin/integrate` (build any app inside xous-core without hand-copying)
  - `bin/lint` (rules for each systemic bug)
  - a 10/10 gate: hosted build, strict clippy, host tests, lint
  - an app template that splits pure logic (`core/`, host-tested) from a thin device shell
  - shared appkit libraries
- **Then the apps, rebuilt in rounds by Claude Sonnet workers:**
  - Each worker had a private build tree.
  - Each round's results were **re-graded independently** before counting.
  - Every round passed re-grade on its first submission.
- **The flywheel:**
  - Every worker wrote a retro.
  - Each retro became a lesson (26 so far, in `LESSONS.md`) or a shared helper before the next round.
- **Concrete examples worth telling:**
  - One sine table, copied into ten generative-art apps, returned `sin(0) = cos(0) = 0`, collapsing every figure onto its centre. It was replaced by one tested fixed-point module (within 1 LSB of f64 at every angle).
  - pet's clock dropped up to a second per sync, so the pet aged about 3% slow. This led to a shared suspend-safe clock with a regression test.
  - Regex Lab was tested against the Rust `regex` crate and found 5 real matching bugs. Penrose now does correct P3 deflation.
  - Workers found a bug in the orchestrator's own toolkit code: unfilled circles erased each other.
- **Honest limits:**
  - Nothing has run in Renode or on hardware yet.
  - Screenshots are the next milestone.
  - Several apps were renamed to stop overclaiming (Net Check is not a network scanner; RSS Viewer doesn't fetch).
- **Angle for CDM readers:** audit, then fix the shared cause, then gate, then delegate with verification. This is the same discipline CDM applies to Salesforce orgs. Keep it to one paragraph; it's a labs post.

### 4.2 Later posts, once screenshots exist

- **Gallery post:** "40 apps on a 1-bit screen", with real Renode captures, one hero shot per app.
- **Generative art on a 1-bit display:** fixed-point trig, Robinson deflation, Gray–Scott on 70 KB, and moiré by XOR.
- **Security apps done honestly:**
  - Key Ceremony, One-Time Pad and Cipher Workshop, including what each does *not* protect.
  - TRNG without modulo bias.
  - Why the LAN transfers use a one-time code and are still plaintext.

---

## 5. Catalog copy (one line per app; source SITE_FACTS §1)

Draft copy, not final: strip any em dashes before shipping. Badge `LAN` = optional local-network feature.

**Productivity**
- Writer: markdown editor, one-entry-per-day journal, and forward-only typewriter mode. `LAN` export with one-time code.
- Flashcards: multi-deck flashcards with spaced repetition. `LAN` deck import/export with one-time code.
- Timers: Pomodoro, lap stopwatch and named countdowns that keep running when you leave the app.
- Calculator: scientific calculator with algebraic and HP-style 4-level RPN modes.
- Day Planner: events, month calendar and tasks.
- Contacts: offline address book.
- Budget Ledger: income and expense ledger with a monthly summary, in exact integer cents.
- Math Drill: timed 10-problem arithmetic quiz with saved bests.
- RSS Viewer: reader for RSS and Atom XML you type in or load from samples. It does not fetch.

**Security**
- Key Ceremony: passwords, PINs, passphrases, BIP-39 mnemonics and key material from the hardware TRNG.
- One-Time Pad: generate pads from the TRNG, share them by reading hex, and encrypt short messages by XOR.
- Cipher Workshop: six classical ciphers with frequency analysis, for learning. None of them protects anything.

**Developer tools**
- QR Code: show text as a QR code a phone can read off the screen.
- Barcode: Code 128, Code 39, EAN-13 and UPC-A on the 1-bit screen, readable by a scanner.
- JSON Viewer: strict JSON validator and collapsible tree.
- Regex Lab: regular-expression workbench with live matches.
- SQL Explorer: in-memory SQL playground. Not SQLite.
- Cron Explainer: explains a cron expression in English and previews the next fire times.
- Git Log Viewer: browse a `git log` pushed over the `LAN` (one-time code).
- Net Check: subnet calculator, port reference and a confirmed, rate-limited TCP connect check (`LAN`). Not a scanner.
- HW Tools: battery, EC, SoC, voltage rails and GPIO readouts.

**Games**
- Othello: Reversi against a CPU opponent.
- Mines: Minesweeper with hardware-random mine placement.
- Decision Engine: dice, coins, cards, spinner, scoreboard and brackets, fair by construction.
- Pet: a virtual pet that keeps living while the app is closed.
- Fortress: a small fortress-building management sim.
- Text Adventure: a small original six-room adventure.
- Carse Automata: a cellular-automaton garden with no score and no end, after James P. Carse's *Finite and Infinite Games*.

**Emulators and debuggers**
- C64: hobby-grade Commodore 64 emulator for text-mode software and BASIC. `LAN` program import (no authentication; see §6).
- Game Boy CPU debugger: step through Sharp LR35902 code. Not a full Game Boy emulator.

**Generative art**
- IFS Fractals: chaos-game fractal attractors.
- Penrose Tiler: true Penrose P3 rhomb tilings.
- Sacred Geometry: ten compass-and-straightedge figures with real circles.
- Kaleidoscope: N-fold mirrored line patterns.
- Moiré Generator: two layers XORed to show interference.
- Lissajous: animated Lissajous curves.
- Spirograph: animated hypotrochoids and epitrochoids that always close.
- Reaction: Gray–Scott reaction-diffusion patterns.
- Life: Conway's Game of Life with glider-tested rules.
- Death: a Life-like automaton explorer with editable B/S rules.

---

## 6. Decisions Tyler must make before publishing

1. **Renames:**
   - Display names: Cron Explainer, Net Check, IFS Fractals, Text Adventure (zork).
   - Package names: rss-viewer, sql-explorer.
   - Whether the site uses repo names or display names.
2. **Local-network transfers:**
   - Is publishing "plaintext over LAN with a one-time code" acceptable for flashcards, writer and gitlog?
   - Should c64's unauthenticated :6464 listener ship as is, or be gated before it's advertised?
3. **Roadmap claims in the launch post:** current status of the LoRa expansion (NextPCB Accelerator application, RAK3172, Meshtastic) and of the C64 game project with Maelee. Should either be mentioned at all now?
4. **Game Boy:** present it as a debugger, or leave it out of the showcase?
5. **Timing:** publish the corrections now (low risk, fixes false claims), and the rebuild post now or after the Renode screenshots?
6. **Hardware facts** in correction 7: confirm the part names before they go on the page.

## 7. Blocked or next

- **Renode captures:** Renode is not installed on this Mac. Every README still says "Renode capture not yet run". Needed for the gallery, the calc screenshots and the README screenshot pass (REMEDIATION checklist items 3–4).
- **Combined 40-app re-grade** (REMEDIATION item 2), to run before quoting "40/40" publicly.
