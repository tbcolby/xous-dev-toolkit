# Games slice audit

Repos: precursor-decide, precursor-mines, precursor-fortress, precursor-pet, precursor-zork, carse-automata (pkg `carse`), precursor-mathdrill, precursor-flashcards.

Method: every `.rs` file in the slice was read in full (about 10k lines). Lint output in `SCRATCH/lint/<pkg>.txt` was cross-checked against upstream `services/gam`, `services/pddb`, `services/trng`, `libs/ux-api` and `apps/ball`. Two findings were also executed on the host with a stub RNG: the decide tournament bug (reproduced) and the flashcards deserializer (see F-2). Nothing in the source repos was modified.

Paths below are relative to `SCRATCH/precursor/<repo>/`.

## Slice summary

| App | Score | State as published on GitHub |
|---|---|---|
| carse | 7 | Only app that is two `unsafe {}` blocks away from building. Real bugs in persistence cadence and rendering. |
| flashcards | 6 | Two drift errors from building. Index corruption on delete, UI-thread-blocking TCP, wrong UX type. |
| decide | 5 | Good pure logic and correct RNG, but tournament is broken for every size above 2, and `ui.rs` cannot compile on any xous-core. |
| mines | 5 | Game logic is sound apart from one borrow error; cannot compile on any xous-core (undeclared crate, missing trait import). |
| mathdrill | 3 | Logic fine, `ui.rs` is written against a GAM API that does not exist, and arrow key codes make the menu unusable on hardware. |
| zork | 2.5 | Never compiled. Parser cannot match most item names, descriptions are clipped, score is farmable. |
| fortress | 2 | Never compiled. Wrong arrow codes, sealed map, deterministic RNG, no persistence. |
| pet | 2 | Never compiled. No persistence, no real-time model, pet dies in about 5 minutes, only "Feed" reachable on hardware. |

Two distinct populations exist in this slice:

- **Group A (carse, flashcards, decide, mines)** follow the upstream `ball` skeleton (`target_os = "none"`, `msg_scalar_unpack!`, `request_content_canvas`, U+2190..2193 arrows).
- **Group B (fortress, pet, zork, and the UI half of mathdrill)** use APIs that I could not find anywhere in upstream and that do not match any xous-core API shape I know of: `gam::RawkeysEvent::from_message`, `Ticktimer::set_alarm`, `gam.clear()`, `gam.draw_textview`, `tv::TextView::new(tb, text).style(..)`, a `Canvas` type, `gam.draw_rectangle(Point, Point)`, `TextBounds::BoundingBox(Point, Point)`, `xous::wait_thread(Duration)`. This is not upstream drift; these apps have very likely never been built. The clone is shallow, so I could not check history, but the working copies on the external drive should be compared before trusting GitHub.

---

## carse (carse-automata)

**Verdict:** best-engineered app in the slice; correct pump/focus pattern, binary PDDB format, diff rendering. Health **7/10**.

### Bugs

| Sev | Location | Description | Failure scenario | Fix |
|---|---|---|---|---|
| High | `src/app.rs:350` | Periodic save is evaluated on every 100 ms pump, not on every tick advance. `self.tick % 50 == 0` stays true while paused. | Pause (or simply release space) when tick is 50, 100, 150...: `save_epoch_state` writes 4 KB and calls `pddb.sync()` ten times per second until the sim resumes. UI stalls, flash wears, battery drains. Also fires continuously after restoring a saved state whose tick is a multiple of 50. | Move the save inside `if self.simulation_running { advance_tick(); if tick % 50 == 0 { save } }`, or track `last_saved_tick`. |
| Medium | `src/app.rs:107-114`, `601-608` | "Drift after 24 h" compares `ticktimer.elapsed_ms()` (milliseconds since boot) across sessions. | `CarseApp::new` runs once at process start, seconds after boot, so `now > last_session_ms + 86_400_000` is essentially never true. The README feature is dead. | Persist wall-clock time (RTC via `llio`/`std::time::SystemTime`) instead of uptime. |
| Medium | `src/rules.rs:39`, `src/app.rs:176` | `collapse_energy` is 40,000..80,000 but maximum grid energy is 4096 x 6 = 24,576. | "Energy Collapse" death can never occur; README and DESIGN describe it as one of three death modes. | Scale the range to something reachable (for example 9,000..20,000), or derive from `CELL_COUNT`. |
| Medium | `src/app.rs:615-677`, `438-467` | `Rectangle` bottom-right is inclusive upstream (`libs/ux-api/src/minigfx/op.rs:173,207`). All cell rects use `bx + cp` / `by + cp`, so every shape is 1 px larger than the comments say and the erase rect is 6x6. | In `draw_diff`, redrawing a changed cell erases the left column and top row of its right and lower neighbours, which are not redrawn unless they also changed. Visible speckle that only clears on a full redraw. Cursor frame is also 6x6. | Use `bx + cp - 1` / `by + cp - 1` and shrink each state's rect by one. |
| Medium | `src/app.rs:357-372`, `438` | Cursor is drawn on top of cells but never erased. `draw_cursor` returns early when `cursor_visible` is false and cursor movement does not mark cells dirty. | With the sim paused, moving the cursor leaves a trail of frames, and the blink never turns off on unchanged cells. | Track previous cursor cell and redraw it (and the current cell when blinking off) with `cell_rects_full`. |
| Medium | `src/app.rs:417-423`, `227-253` | Diff rendering compares against `prev_grid`, but `touch_cell` mutates `grid` without touching `prev_grid`. | Cell went 3 -> 2 on the last tick; user touches it back to 3; it now equals `prev_grid`, so the diff skips it and the screen keeps showing state 2. | Keep a separate "last drawn" grid, or push the touched cell into a dirty list. |
| Medium | `src/storage.rs:86-99`, `src/app.rs:524` | `load_epoch_state` copies cell bytes unvalidated; `STATE_NAMES[cell_state as usize]` indexes a 7-element array. No magic or version on any record. | A corrupted or future-format `epoch.current` with a byte above 6 under the cursor (32,32) panics in `draw_status` at every launch until the key is deleted by hand. | Validate `cells.iter().all(|&c| c <= 6)` on load, add a magic/version header, use `.get()` for the name lookup. |
| Medium | `src/storage.rs:40-51`, `53-67` | Touch map is written and read as 4096 separate 4-byte `write_all`/`read_exact` calls on a `PddbKey`. Each is an IPC round trip. | `save_state` runs on every focus loss and blocks the main loop for thousands of IPC calls plus three `sync()`s; same cost at boot. | Serialize into one 16 KB `Vec<u8>` and do a single write/read. Sync once at the end of `save_state`. |
| Medium | `src/storage.rs:34` | `pddb.try_mount()` in the constructor. Upstream documents that this "may cause a password request box to pop up" (`services/pddb/src/lib.rs:209-216`). The app process starts at boot. | Suspected: the app races the system mount flow at boot and can trigger its own PIN prompt; if mount is declined the app silently starts fresh and later overwrites nothing useful. | Use `is_mounted_blocking()` (other apps in the slice do), ideally lazily on first focus rather than in `new`. |
| Low | `src/storage.rs:137-183` | Fossil slots are reused modulo 100 and PDDB keys do not shrink on a shorter write. | After epoch 100, a fossil whose RLE is shorter than the previous occupant keeps stale trailing pairs; `rle_decompress` yields more than 4096 bytes and the fossil renders blank. | Store the compressed length in the header and read exactly that many bytes, or `delete_key` first. |
| Low | `src/app.rs:358-360`, `530-531` | In scrub mode `draw_fossil_view` runs on every pump and reloads the fossil from PDDB each time. If the slot is missing nothing is drawn. | Holding space+arrow costs a PDDB read, RLE decode and full grid draw ten times a second; scrubbing past recycled epochs shows a stale frame with no message. | Cache the loaded fossil keyed by `scrub_epoch`; draw a "no record" line. |
| Low | `src/grid.rs:50-80`, `src/app.rs:680-693` | One blocking TRNG IPC per cell (4096 per seeding). `unwrap_or(0)` hides TRNG failure. `% 100`, `% 3`, `% 20` have negligible modulo bias, and three decisions reuse bits of one `r`. | Slow epoch start; cosmetic bias only. | `trng.fill_buf` into a `[u32; N]` once. |
| Low | `src/app.rs:291-303` | A hold always begins with a "fresh press", so starting the simulation always perturbs the cursor cell and increments its permanent touch count. | Every run alters the touch map. | Defer the touch until the key is known not to be a hold, or use a separate run key. |

### Upstream drift

`xous::destroy_server` is now `unsafe` (`src/main.rs:123`, `168`). Wrap in `unsafe {}`. That is the only compile blocker.

### Improvements

1. Redraw only when something changed. `draw_status` clears and reposts three TextViews and calls `gam.redraw()` every 100 ms even when paused; cursor blink needs 2 Hz at most.
2. `CarseApp` holds three 4 KB grids and a 16 KB touch map by value, and `new()` moves several copies through tuples and `unwrap_or([0u32; CELL_COUNT])`. Box them to keep stack use predictable.
3. `advance_tick` clones the grid three times per tick; swap buffers instead.
4. No unit tests for `rules::transition`, `rle_compress`/`rle_decompress`, or `Grid::neighbors`, all of which are pure and host-testable.

---

## flashcards (precursor-flashcards)

**Verdict:** works in the emulator (screenshots exist), but deck deletion corrupts the index and network import/export freezes the app. Health **6/10**.

### Bugs

| Sev | Location | Description | Failure scenario | Fix |
|---|---|---|---|---|
| High | `src/storage.rs:147-157`, `72-80` | `write_index` rewrites `_index` from offset 0 with no truncation, and PDDB keys do not shrink (`PddbKey` has no truncate/`set_len` upstream). | Index is `A\nB\nC`. Delete C: 3 bytes `A\nB` are written over a 5-byte key, which still reads `A\nB\nC`. The deleted deck reappears as a 0-card ghost that cannot be opened. Delete B instead: key becomes `A\nC\nC`, a duplicate entry. | `delete_key(DICT, INDEX_KEY)` before rewriting, or store a length prefix, or drop the index and use `pddb.list_keys(DICT)`. |
| High | `src/main.rs:563-564`, `459-463`, `260-264`; `src/import.rs:62-75` | `listen_for_import` is called on the main event loop and does a blocking `accept()` with no timeout. The comment says "Set a timeout so we can check for cancellation" but the code calls `set_nonblocking(false)`. | Pressing `i` freezes the app until some host connects. The on-screen "q=cancel" and the `ImportWait` key handler (`main.rs:222-228`) are unreachable. Redraw and focus messages pile up in the server queue; `handle_f4` admits "Can't interrupt blocking import". With no Wi-Fi the only exit is a reboot. | Run the listener in a worker thread that posts an `ImportDone` message to the main SID; use a non-blocking listener polled with a deadline, and honour cancel. |
| High | `src/import.rs:127-152`, `src/main.rs:301-308`, `540-548` | Export has the same blocking `accept()` on the UI thread, with no waiting screen at all. | Pressing `e` in the deck menu appears to hang the app with the old screen still showing. | Same worker-thread pattern; draw a status screen. |
| Medium | `src/main.rs:73` | Registered as `UxType::Chat` with `rawkeys_id`. Chat layout creates an IME input area and GAM also feeds keys to the IME. | Confirmed by the repo's own `screenshots/deck_list.png`: the predictor bar is on screen and the typed key `a` is echoed in the input field below the app. Vertical space is lost and every keystroke shows up there. | `UxType::Framebuffer`, as in upstream `apps/ball` and in carse/decide. |
| Medium | `src/import.rs:63`, `132` | Listeners bind `0.0.0.0` with no authentication or peer check. | While import is waiting, any host on the network can inject a deck. While export is waiting, whoever connects first receives the deck contents, which live in the encrypted PDDB. A slow sender (1 byte every 9 s) holds the 10 s per-read timeout open for days because there is no overall deadline. | Show and confirm the peer address, add an overall deadline, consider a one-time code displayed on screen. |
| Medium | `src/deck.rs:34-35` | `Vec::with_capacity(count)` with `count` read straight from stored bytes. | A corrupted deck key with a large count panics with capacity overflow on the 32-bit target (or aborts on allocation failure) every time the deck is opened. On the 64-bit host the same input returned `None`, so this only shows on device. | Clamp: `count.min((data.len() - 4) / 4)`. |
| Medium | `src/main.rs:566-570`, `src/storage.rs:48-68` | Deck name comes from the network with no length or character limit. Key name is `deck.<name>` and upstream `KEY_NAME_LEN` is 95. | A `#name:` longer than 89 bytes makes `pddb.get` fail; the error is logged but the index is still updated, creating a ghost deck. `#name:` with an empty value yields a deck whose index line is filtered out, so the cards are saved but never listed. | Validate and truncate the name (for example 1..=48 printable chars) and only update the index after a successful write. |
| Medium | `src/storage.rs:15` | `try_mount()` in the constructor at process start; `ensure_demo_deck` then runs immediately. | Suspected: same boot-time PIN prompt race as carse. If the mount is not available, `has_index` fails and the app tries to create the demo deck on every boot. | `is_mounted_blocking()`, deferred to first focus. |
| Medium | `src/main.rs:599-614` | Shuffle uses an LCG and takes `rng % (i + 1)`, i.e. the low bits, which in a power-of-two LCG have very short periods (bit 0 alternates). | Shuffles are visibly patterned and far from uniform; no TRNG is used even though the device has one. | Add the `trng` dependency and reuse the rejection-sampling `Rng` from decide/mines. |
| Low | `src/ui.rs` (all `.expect(...)`/`.unwrap()` on GAM calls) | Every draw call panics on error. | Any transient GAM error (for example a draw while losing focus after `do_import` returns, `main.rs:579`, which ignores `allow_redraw`) kills the app. | `.ok()` and log, as the other apps do. |
| Low | `src/ui.rs:146-160` | Card text is posted into a fixed box with no pagination and `tv.overflow` is ignored. | Imported cards longer than the box are silently clipped. Upstream `post_textview` also does `String::truncate(3072)`, which panics if byte 3072 is not a char boundary, reachable with a long non-ASCII card. | Cap field length at import (well under 3072 bytes, on a char boundary) and add paging. |
| Low | `src/import.rs:87-100` | When the 64 KB cap is hit mid-line the partial last line is kept as a card. | Truncated final card, possibly ending in U+FFFD. | Drop the last line if the read stopped on the cap. |
| Low | `src/deck.rs:21-23` | Field lengths are cast to `u16` without a check. | Safe today only because import caps total input at 65,536 bytes; any other writer would silently corrupt the deck. | `u16::try_from` and reject. No format version byte exists either. |

### Upstream drift

- `xous::create_server_id().unwrap().0[0]` (`src/main.rs:601`): `SID`'s field is private now; use `.to_array()[0]`, or better, the TRNG.
- `xous::destroy_server` is `unsafe` (`src/main.rs:686`).

### Improvements

1. `list_decks` opens every deck key to read its count on each refresh; cache counts in the index.
2. No progress tracking or spaced repetition; "Next" stops at the last card with no end-of-deck summary.
3. `deck::serialize_cards`/`deserialize_cards` and `import::parse_tsv` are pure and should have host tests (round trip, truncated input, oversized count).

---

## decide (precursor-decide)

**Verdict:** the most complete feature set and a correct unbiased RNG, but the tournament tool is wrong for every bracket larger than two players and the published source cannot compile. Health **5/10**.

### RNG fairness (asked for specifically)

`src/rng.rs:27-40` uses rejection sampling with `threshold = u32::MAX - (u32::MAX % max)`. The accepted range `[0, threshold)` is an exact multiple of `max`, so `val % max` is uniform. Dice (`roll_die`), cards (Fisher-Yates in `shuffle`, `j` drawn from `[0, i]`), 8-ball, weighted spinner and coin flip (`u32 & 1`) are all unbiased. The one weakness is failure handling, listed below.

### Bugs

| Sev | Location | Description | Failure scenario | Fix |
|---|---|---|---|---|
| High | `src/tournament.rs:148-186` | `propagate_winners` auto-advances a later-round match whenever exactly one slot is filled, treating "opponent not decided yet" as a BYE. Reproduced on the host with a stub RNG. | 4 or 8 players: after the first match is decided, its winner is copied into the next round, auto-wins it, and is declared champion after a single decision. 3 or 5 players: the BYE player has already "won" the final inside `new()` and becomes champion as soon as any match is recorded. Only a 2-player bracket works. | Auto-advance only when the feeder match for the empty slot is a true BYE (both of its slots empty and no pending feeders). Simplest: mark BYE matches explicitly at construction and propagate that flag. |
| Medium | `src/rng.rs:22-24` | `get_u32().unwrap_or(0)`. | If the TRNG call fails, every die rolls 1, every coin lands tails, shuffles become the identity, and nothing is shown to the user. For an app whose premise is hardware randomness this is a silent fairness failure. | Return `Result`, surface an error state in the UI, or retry and panic rather than fabricate. Same code is copied into mines and mathdrill. |
| Medium | `src/app.rs:246-249`, `src/storage.rs:41-57` | `go_home` calls `save_state`, which performs four key writes and four `pddb.sync()` calls on every return to the menu, even when nothing changed. | Noticeable pause on every `q`, plus needless flash wear. | Dirty flags per key and one sync. |
| Medium | `src/spinner.rs:41-47` | `remove_segment` refuses when two or fewer segments remain, and segments persist. | Once two segments exist they can never be cleared; a typo in the first two entries is permanent. | Allow removal down to zero; `can_spin` already guards spinning. |
| Low | `src/turns.rs:83-88`, `53-81` | `skip_current` flags the current player and advances, but flags are cleared at the round boundary before that player comes up again. | "Skip" behaves exactly like "Next"; the `[SKIP]` marker never causes a skip. | Decide the intent (skip next turn) and keep the flag until it is consumed. |
| Low | `src/turns.rs:41-51` | `remove_player` does not adjust `current` when removing an index below it. | Removing an earlier player silently advances the turn by one. Only reachable via API today since the UI removes the current player. | Decrement `current` when `index < current`. |
| Low | `src/app.rs:331-337` | `finalize_tournament` takes the names, and `Tournament::new` returns `None` for more than 16 players. | Entering 17 names discards all of them with no message. | Enforce the cap during entry. |
| Low | `src/coin.rs:64-65`, `104-106`; `src/app.rs:295-301` | Byte slicing of user-entered names (`label[..9]`, `name[..1]`); text input accepts any non-control char and measures length in bytes. | Suspected (needs a non-ASCII keyboard layout): a multi-byte first character or one straddling byte 9 panics on the next flip. | Use `chars()`. |
| Low | `src/app.rs:155-166`, `src/storage.rs:61-65` | `last_tool` is saved but `load_last_tool` is never called. | Dead feature; one wasted write and sync per save. | Use it in `init_storage` or remove it. |
| Low | `src/dice.rs:137` | Crit detection uses `rolls[0] == 20` after exploding has been applied. | An exploding d20 that rolls 20 never reports a critical. | Record the natural first roll. |
| Low | `src/ui.rs` (dice, card, coin, 8-ball art) | ASCII art relies on space alignment but is drawn with proportional `GlyphStyle::Regular`. | Boxes and pips are misaligned. | `GlyphStyle::Monospace`, or draw with rectangles and circles. |

### Upstream drift and build

- `ui.rs` uses `graphics_server::` throughout but `graphics-server` is not in `Cargo.toml`. This fails on any xous-core version. On current upstream the types are re-exported from `gam` (`gam::Rectangle`, `gam::DrawStyle`, `gam::PixelColor`, `gam::Gid`).
- `ui.rs` uses `alloc::format!` but `main.rs` has no `extern crate alloc;` (other modules declare it locally, which only scopes it to that module).
- `write!(tv, ...)` without `use core::fmt::Write` (7 sites).
- `PddbKey::set_len` (`src/storage.rs:54`) does not exist upstream and I found no evidence it ever did.
- `xous::destroy_server` is `unsafe`.
- `gam::APP_NAME_DECIDE` requires a manifest entry that the README does not document; the local `APP_NAME` const is unused.

### Improvements

1. Unit tests for `dice`, `cards`, `tournament`, `turns`, `spinner` behind a small `RandomSource` trait. The tournament bug would have been caught by a three-line test.
2. Spinner weights, turn tracker and tournament state are not persisted, while scoreboard and segment names are.
3. A 16-player bracket is about 25 lines and overflows the 400 px text block with no scrolling.
4. Hardcoded 336x536 layout; the canvas is shorter than the screen (status bar). Use the `screensize` that `main.rs` already fetches and only logs.

---

## mines (precursor-mines)

**Verdict:** rules are implemented correctly (first-click safety, flood fill, win detection); blocked by one borrow error plus undeclared imports. Health **5/10**.

### Logic review (asked for specifically)

- **First-click safety:** `place_mines` (`src/minefield.rs:130-164`) excludes the 3x3 around the first reveal, shuffles the remaining positions with unbiased Fisher-Yates and takes the first `mine_count`. Correct for all three difficulties (smallest free pool is 72 for 10 mines).
- **Flood fill:** iterative with an explicit stack, re-checks `Hidden` on pop so duplicates are harmless, and does not open flagged cells. Correct once the borrow error is fixed.
- **Win check:** counts revealed safe cells; correct.

### Bugs

| Sev | Location | Description | Failure scenario | Fix |
|---|---|---|---|---|
| High (build) | `src/minefield.rs:242-253` | `let cell = self.cell(cx, cy)` holds a shared borrow that is used again at line 251 after `self.cell_mut(..)` and `self.cells_revealed += 1` (E0502, E0506). The line-251 check is also dead: the cell was just proven `Hidden`. | Does not compile on any toolchain. | `if self.cell(cx, cy).state != CellState::Hidden { continue; }` and delete lines 250-253. |
| Medium | `src/main.rs:95-112`, `src/app.rs:107-117`, `235-245` | `check_best_time` runs inside `handle_key`, before `update_timer(now_ms)`, and once the state is `Won` the timer no longer updates. | The recorded and displayed winning time is the elapsed time as of the previous keypress, so best times are understated by however long the last move took. | Call `update_timer(now_ms)` before `handle_key`, or pass `now_ms` into `check_best_time`. |
| Medium | `src/main.rs:79-86` | There is no timer pump; `elapsed_seconds` only changes on a keypress or a GAM-initiated redraw. | The on-screen clock is frozen while the player thinks. README says the timer "displays seconds". | Add a 1 Hz pump thread using the `ball` pattern (carse already has it), stopped on background. |
| Medium | `src/main.rs:50` | `UxType::Chat` with raw keys. | IME bar and key echo as shown in the flashcards screenshot; the 46 px footer drawn at y=490..536 falls outside the content canvas. | `UxType::Framebuffer` and lay out from `get_canvas_bounds`. |
| Medium | `src/ui.rs:216-227`, `271-398` | Every keypress redraws the whole board: one `draw_rectangle` IPC per cell plus one `post_textview` per number. | Expert (16x20) is 320 rectangles and up to about 250 TextViews per cursor move on a 100 MHz CPU; cursor movement will lag badly. | Redraw only the old and new cursor cells on a move; batch with `GamObjectList` as carse does. |
| Low | `src/app.rs:193-201` | Timer starts whenever Enter is pressed in `Ready`, even if the cell is flagged and `reveal` returns false. Mines are also placed at that point. | Clock runs before any cell is opened. | Start the timer only when `reveal` returns true. |
| Low | `src/app.rs:107-112` | Elapsed time is wall-clock from `game_start_ms`; backgrounding the app does not pause it. | Switching apps mid-game inflates the time. | Accumulate elapsed only while focused. |
| Low | `src/minefield.rs:306-316` | On loss only mines are revealed. | Incorrect flags are not marked, which is standard Minesweeper feedback. | Mark wrong flags. |
| Low | `src/storage.rs:38-48` | Same write-without-truncate pattern as elsewhere once `set_len` is removed. | `best_times` JSON that gets shorter (for example `{"beginner":100}` to `{"beginner":99}`) leaves a trailing byte; `serde_json` rejects trailing characters, all best times read as `None`, and the next save overwrites the others. | Delete the key before writing. |
| Low | `src/rng.rs:17-19` | `unwrap_or(0)` on TRNG failure. | Shuffle becomes deterministic; mines always land in the same cells. | See decide. |

### Upstream drift and build

Same set as decide: undeclared `graphics_server`, missing `use core::fmt::Write` (7 sites in `ui.rs`), nonexistent `PddbKey::set_len`, `destroy_server` now `unsafe`. `APP_NAME = "Mines"` is used directly and there is no manifest snippet in the README.

### Improvements

1. `minefield.rs` only needs a shuffle function; abstract the RNG and add host tests (first-click zone, flood fill, win count).
2. No chord/auto-reveal on numbered cells; no confirmation before `q` or `r` discards a game in progress.
3. Game in progress is not saved on background or quit.

---

## mathdrill (precursor-mathdrill)

**Verdict:** state machine and problem generator are fine; the UI module targets a nonexistent API and the key constants are from the wrong platform. Health **3/10**.

### Bugs

| Sev | Location | Description | Failure scenario | Fix |
|---|---|---|---|---|
| High (build) | `src/ui.rs` (whole file) | Uses `Canvas`, `gam.draw_textview`, `tv::TextView::new(tb, text).style(..).invert(..)`, `Rectangle::style(..)` as a method, `gam.draw_line(canvas, Point, Point, style)`, `i16` coordinates. None exist upstream. | About 50 errors that no version pin will fix; this file has to be rewritten against the `TextView::new(gid, bounds)` + `post_textview` pattern used by decide/mines. | Port from `decide/src/ui.rs` helpers. |
| High | `src/app.rs:19-22` | Arrow keys are defined as U+F700..F703 (macOS function-key codes). Precursor sends U+2190..2193, as decide/mines/carse use. | Initial menu focus is Operation; Up/Down never match, and Enter on Operation only cycles the operation. The Start row cannot be reached, so a quiz cannot be started on hardware. | Use the U+2190..2193 constants. |
| Medium | `src/app.rs:205-212` | "New best" is an OR of three independent criteria and then overwrites the whole record. | Previous best: 9/10 with streak 5. New session: 6/10 with streak 6. `best_streak > prev.streak` is true, so the stored best becomes 6/10. Best scores can go down. | Track each metric separately (max correct, max streak, min avg time at max correct). |
| Medium | `src/main.rs:116-117`, `src/app.rs:179-193`, `335-347` | `check_feedback_timeout` is only called from the key handler. | Feedback never auto-advances despite the footer saying "auto-advances in 1.5s". After 1.5 s the next key (for example the first digit of the next answer) is swallowed by `handle_feedback` and then triggers the advance, so the digit is lost. | Add a one-shot timer message, or drop the claim and accept any key. |
| Low | `src/main.rs:143-150`, `src/ui.rs:168-189` | On regaining focus in `BestScores` the code calls `ui::draw`, which renders the placeholder "(Select to see stats)" screen instead of `draw_best_with_data`. | Placeholder text appears after an app switch. | Route through the same branch as the Redraw handler; delete `draw_best_scores`. |
| Low | `src/app.rs:164-165` | Per-problem time is wall-clock and includes time in the background. | Average time is inflated after an app switch. | Pause on focus loss. |
| Low | `src/storage.rs:60-70` | Best scores are keyed by difficulty only. | Easy addition and Easy mixed share one record. | Include the op mode in the key. |
| Low | `src/main.rs:122-124` | Three PDDB reads on every keypress while in `BestScores`. | Sluggish; unnecessary. | Load once on entry. |

### Upstream drift

`PddbKey::set_len` (nonexistent), `destroy_server` `unsafe`, `gam::APP_NAME_MATHDRILL` needs an undocumented manifest entry. Everything else in `ui.rs` is not drift.

### Improvements

1. `problems::generate` is pure given an RNG; add tests (subtraction never negative, division exact, ranges).
2. Operation and difficulty selections are not persisted (`save_state` is an empty function).
3. Easy division frequently produces `n / 1`.

---

## zork (precursor-zork)

**Verdict:** a 6-room demo with a one-word parser; never compiled. Health **2.5/10**.

### Bugs

| Sev | Location | Description | Failure scenario | Fix |
|---|---|---|---|---|
| High (build) | `src/main.rs:1-2`, `17`, `29-31`; `src/ui.rs` | `cfg_attr(target_os = "xous", no_std/no_main)` makes the crate `no_std` + `no_main` on the real target (upstream uses `target_os = "none"`), where `world.rs:197` `vec!` is then not in scope and there is no entry point. Plus `gam::RawkeysEvent`, `xous::wait_thread(Duration)`, `gam.clear()`, two-`Point` `draw_rectangle`, `TextView::new(bounds)` with `tv.text = String`. The GAM token is discarded and no content canvas is ever requested. | Cannot build for host or device. | Rebase `main.rs` and `ui.rs` on the decide/mines skeleton. |
| High | `src/world.rs:96-105`, `123-125`, `184-189` | Parser passes only `parts[1]` and matches it as a prefix of the full item name. | `take key`, `take sword`, `take coins`, `take orb`, `take the crown`, `x coins` all answer "I don't see that here." The player must type `take brass`, `take silver`, `take gold`, `take crystal`. README advertises "partial name matching". | Join the remaining words, strip articles, and match if any word of the item name starts with the noun. |
| Medium | `src/ui.rs:50-57` | Each output entry is drawn in a single 17 px TextView; room descriptions are 80 to 95 characters. | Descriptions are cut off after roughly one line of Small text; the core content of a text adventure is unreadable. | Word-wrap into display lines when pushing to `output`, or use one growable TextView per entry. |
| Medium | `src/world.rs:123-147` | `take` always adds 5 points; `drop` does not subtract. | `take brass` / `drop brass` repeated yields unlimited score. | Score once per item (flag on `Item`). |
| Medium | `src/app.rs:62-73`, `src/main.rs:31` | `quit` sets `game_over`, and `handle_key` then returns false, which terminates the process. | "Thanks for playing!" is never displayed and the app cannot be reopened until reboot. | Stay in a game-over state and offer restart. |
| Medium | `src/world.rs:36`, `85` | `output` grows without bound. | Every command appends 1 to 8 `String`s forever on a small-heap device. | Cap at a few hundred lines (`VecDeque`). |
| Low | `src/app.rs:11-12`, `76` | Scroll keys use U+F700/F701 and `output_scroll` has no upper bound. | Scrolling never triggers on hardware; if it did, over-scrolling would need the same number of presses to come back. | Correct key codes; clamp to `total - max_lines`. |
| Low | `src/app.rs:15`, `86-92` | `AppState::Help` is never entered. | Dead code. | Remove. |
| Low | `src/world.rs:86` | `moves` increments for unknown commands, `help` and `score`. | Move counter is inflated. | Count only successful actions. |
| Low | `README.md` | Claims "7 collectible items"; there are 6 portable items. No win condition; the Brass Key opens nothing. | README vs reality. | Fix the README or add the puzzle. |

### Improvements

1. A text adventure is the one app in this slice where `UxType::Chat` with `gotinput_id` (line input via the IME) is the right design. Use it properly instead of raw keys plus a hand-drawn prompt.
2. No save/restore of room, inventory or score.
3. `process_command` is pure; add parser tests.

---

## fortress (precursor-fortress)

**Verdict:** an idle simulation with a hand-drawn map; never compiled and not playable on hardware as written. Health **2/10**.

### Bugs

| Sev | Location | Description | Failure scenario | Fix |
|---|---|---|---|---|
| High (build) | `src/main.rs`, `src/ui.rs` | Same nonexistent API family as zork, plus `tt.set_alarm(sid, 2000, op, false)` which is not a ticktimer API. `target_os = "xous"` cfg as in zork. | Cannot build. | Port to the carse pump-thread pattern. |
| High (build) | `src/world.rs:374-376`, `558` | Genuine borrow errors: `self.rand()` is called inside the argument of `self.dwarves.push(Dwarf { .. })`, and `self.add_message(..)` is called while `d` (from `self.dwarves.iter_mut().find(..)`) is still live. | E0499 on any toolchain. | Compute the random values into locals first; copy `d.name` (it is `&'static str`) and end the borrow before `add_message`. |
| High | `src/app.rs:8-11` | Arrow keys are U+F700..F703. | The map cursor, dwarf list, build menu and log cannot be navigated on hardware. Only Bed (index 0) can be built, and only at the initial cursor tile. | U+2190..2193. |
| Medium | `src/world.rs:238-331` | Room geometry: every door opens onto a wall or rock. Hall right wall is at x=32 and the workshop door is at (33,7); bedroom doors at x=8 face the hall wall at x=9 or rock; storage door (25,11) sits under the hall's bottom wall; hall doors (20,5),(21,5) open onto rock at y=4; the farm is surrounded by rock. | All rooms are sealed from one another. Idle dwarves can only wander inside the room they spawned in. | Offset the rooms so they share walls with doors in them, or carve corridors. |
| Medium | `src/main.rs:36-43` | Focus changes are ignored and the tick alarm is re-armed unconditionally. | The simulation keeps running and redrawing every 2 s while the app is in the background; resources are consumed and the battery drains. | Track focus; stop the pump on background as carse does. |
| Medium | `src/world.rs:192`, `200-203` | LCG seeded with the constant 42; no TRNG dependency. | Every game has identical trees, skills and event sequence. | Seed from the TRNG. |
| Medium | `src/world.rs:444-457` | Hunger and tiredness overwrite the dwarf's assigned job with Eating/Sleeping, and both end in `Idle`. | A dwarf assigned to Mining stops mining permanently after its first meal or nap. | Store `assigned_job` separately and restore it. |
| Low | `src/world.rs:125-131`, `465`, `553` | Resources are `u16` with unchecked `+=`. | With seven miners, stone overflows in roughly 15 hours of run time (panic in debug, wrap to 0 in release). | `saturating_add`. |
| Low | `src/ui.rs:72-73`, `src/app.rs:83` | The draw loop stops at 26 rows but `MAP_H` is 28 and the cursor can move to rows 26 and 27. | Cursor disappears off the bottom of the map. | Clamp or shrink the tile height. |
| Low | `src/ui.rs:86` | Map rows are drawn with proportional `GlyphStyle::Small`. | Columns do not line up; the "8x16 monospace" comment is not true for that font. | `GlyphStyle::Monospace`. |
| Low | `src/app.rs:36-43`, `129-146`; `src/ui.rs:197` | The build menu shows `[b] [t] [c] ...` hotkeys that `key_build` does not handle. Anvil has a cost but is not in the menu, so metal and smelting have no use. Farming and Building jobs do nothing; Hauling produces drinks. `z_level` and Stairs are unused. | README features that are not implemented. | Implement or remove. |
| Low | `src/world.rs:441-512` | Hunger saturates at 255 with no consequence; there is no death or loss condition. | Food can sit at zero indefinitely. | Add consequences or document it as a sandbox. |

### Improvements

1. No PDDB dependency, so the fortress is lost on every quit or reboot.
2. Menu key on the map view terminates the process (see cross-cutting).
3. `World::step` is pure and should be tested for bounds (resources, stats, dwarf positions) over a few hundred thousand ticks.

---

## pet (precursor-pet)

**Verdict:** the pet model is small and tidy, but there is no persistence and no real-time model, and it shares fortress's non-compiling shell. Health **2/10**.

### Timekeeping review (asked for specifically)

- Age and all decay are counted in 3-second "ticks" driven by `tt.set_alarm` (`src/main.rs:24`, `39`), which does not exist in the ticktimer API. There is no use of the RTC or of elapsed time.
- **Backgrounded:** focus changes are ignored (`src/main.rs:36`), so the pet keeps ticking and the UI keeps redrawing while another app has focus.
- **Device asleep:** nothing compensates for suspended time; the pet simply freezes and resumes.
- **Reboot or quit:** there is no PDDB dependency. The pet is gone.
- **Pace:** hunger rises 2 per tick, reaches 100 in about 2 minutes, health then falls 2 to 4 per tick. An unattended pet starves roughly 5 minutes after its last meal. The whole life cycle (Egg to Senior) is 300 ticks, 15 minutes.

### Bugs

| Sev | Location | Description | Failure scenario | Fix |
|---|---|---|---|---|
| High (build) | `src/main.rs`, `src/ui.rs` | Same nonexistent API family and `target_os = "xous"` cfg as fortress/zork. | Cannot build. | Port to the carse skeleton. |
| High | `src/app.rs:9-12` | Arrow keys are U+F700..F703. | In the action menu Up/Down never match, so only "Feed" (index 0) can be selected. Play, Sleep, Medicine, Clean and Discipline are unreachable, which means a sick pet cannot be cured. | U+2190..2193, plus number-key shortcuts. |
| High | whole app | No persistence and tick-count time. | See the timekeeping section: a virtual pet that cannot survive being put in a pocket and is erased by a reboot. | Store the pet plus a wall-clock timestamp in PDDB; on focus or boot compute elapsed real time, apply decay in bulk with a cap, and slow the rates to hours. |
| Medium | `src/app.rs:86-93` | After death there is no way to start a new pet. | The only option is Menu, which terminates the process; the app is then unusable until reboot. | Offer "new egg" when `!pet.alive`. |
| Medium | `src/main.rs:36-40` | Tick continues and redraws when unfocused. | Pet starves while the user is in another app; wasted CPU. | Stop the pump on background and reconcile by elapsed time on foreground. |
| Low | `src/app.rs:28`, `48`, `111` | `messages` grows by one `String` per action or event and only the last is ever shown. | Slow unbounded heap growth. | Keep a single `Option<String>`. |
| Low | `src/pet.rs:40-41`, `168-174` | `weight` and `discipline` are tracked and displayed but affect nothing. Seniors never die of age. | Dead mechanics. | Wire them in or remove them. |
| Low | `src/pet.rs:161-165` | `clean` works on a dead pet or an egg; the other actions do not. | Inconsistent. | Same guard as the others. |

I checked the `u8` arithmetic in `pet.rs` for overflow: all additions are bounded before the add (`hunger < 100` then `+2`, `happiness + 25 <= 125`, and so on), and `sick_ticks` cannot reach 255 because health falls below 10 within 40 sick ticks, which triggers death.

### Improvements

1. `Pet::tick` and the actions are pure; add host tests, including a long-run test for the bounds above.
2. Sprite art uses proportional `Regular` glyphs; draw it as a bitmap or use `Monospace`.

---

## Cross-cutting findings

1. **Half the slice has never been compiled.** fortress, pet, zork and mathdrill's `ui.rs` target an API surface that is absent from upstream and internally inconsistent with the other four apps. Separately, decide and mines reference a `graphics_server` crate that is not in their `Cargo.toml`, and call `write!` on a `TextView` without importing `core::fmt::Write`; those fail regardless of xous-core version. mines and fortress also contain genuine borrow-check errors. Only carse and flashcards are within a couple of mechanical fixes of building against current upstream. Worth confirming whether the external-drive copies differ.

2. **`PddbKey::set_len` does not exist, and removing the call exposes a real bug.** decide, mines and mathdrill all end `write_key` with `handle.set_len(..)`. PDDB keys never shrink on a shorter write, so once that line is deleted to make the build pass, any JSON value that gets shorter leaves trailing bytes and `serde_json` rejects the whole record (best times and best scores silently reset). flashcards already suffers the same class of bug in its index, and carse in fossil slots. One shared helper that does `delete_key` then write then a single `sync` would fix all five apps.

3. **No format versioning or validation anywhere.** JSON apps tolerate garbage by returning `None`; the binary formats (carse, flashcards) trust stored counts and cell values and can panic on corrupted data at every launch. Add a magic/version byte and bounds checks.

4. **Wrong arrow key codes in four apps.** fortress, pet, zork and mathdrill use U+F700..F703; the toolkit's own notes and the other four apps use U+2190..2193. In three of them this makes the app unusable on hardware.

5. **`UxType::Chat` for raw-key apps.** mines, flashcards, fortress, pet and zork register as Chat. The flashcards screenshot shows the consequence: IME bar on screen and keystrokes echoed into it. Use `Framebuffer` (decide, mathdrill and carse do), except zork, which should use Chat properly with `gotinput_id`.

6. **Quit paths terminate the process.** decide, mines, mathdrill, flashcards, fortress, pet and zork all break out of the main loop and call `terminate_process` on `q`, F4 or the Menu key. No upstream app does this; apps are started at boot, so as far as I can tell a terminated app stays in the menu but cannot be reopened until reboot. Suspected rather than tested on hardware. Prefer `gam.relinquish_focus()` or simply no quit.

7. **TRNG failure is masked.** The `Rng` wrapper copied into decide, mines and mathdrill, and the direct calls in carse, all use `unwrap_or(0)`. The rejection-sampling code itself is correct and unbiased. The two apps that should use the TRNG and do not are flashcards (weak LCG shuffle) and fortress (constant seed).

8. **Hardcoded 336x536 layouts.** decide, mines, mathdrill, fortress, pet and zork draw footers at y around 490..536 while the content canvas starts below the status bar and is shorter than the screen. carse and flashcards size from `get_canvas_bounds` and are the model to follow.

9. **Background behaviour.** Only carse stops its pump on focus loss. fortress and pet keep simulating and drawing; mines and mathdrill keep their clocks running.

10. **Redraw cost.** decide, mines and the Group B apps clear and redraw the full screen on every key, one IPC per primitive. carse's `GamObjectList` batching and diffing is the pattern to share.

11. **Duplicate code worth a shared crate.** `Rng` (three identical copies), PDDB `read_key`/`write_key` (three copies), header/footer/text helpers (five variants), key constants, and the main-loop skeleton. A small `precursor-app-kit` crate would also be the natural place to fix items 2, 4, 5, 7 and 8 once.

12. **No tests.** There is not a single `#[test]` in the slice, although `minefield`, `dice`, `cards`, `tournament`, `turns`, `problems`, `pet`, `world` (both), `rules`, `deck`, `parse_tsv` and the RLE codec are all pure logic that runs on the host once the RNG is behind a trait. No repo ships a `Cargo.lock`, and six of eight READMEs omit the `apps/manifest.json` snippet their code depends on.
