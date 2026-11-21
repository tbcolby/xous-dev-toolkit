# Devtools slice: bug and improvement report

Repos: precursor-json (`json-explorer`), precursor-regex (`regex-tester`), precursor-rss (`rss-reader`), precursor-sqlite (`sqlite`), precursor-cron, precursor-gitlog, precursor-netscan, precursor-hwtools.

Method: every `.rs` file in the slice was read in full (about 5,000 lines). The pure-logic modules (`parser.rs`, `engine.rs`, `rss.rs`, `sql.rs`, `cron.rs`, `gitlog.rs`, `scanner.rs`) were also compiled unmodified into a host harness (`SCRATCH/devtools_harness/main.rs`, `#[path]` includes, sources untouched) and run. Findings marked **(ran)** were reproduced there; the rest were verified by reading the code and the upstream services (`services/gam`, `services/keyboard`, `services/llio`, `services/com`). Anything not confirmed is marked **Suspected**.

Three things to know before the per-app detail:

1. **Seven of the eight apps are demos, not tools.** There is no network code in `netscan` (it prints six hard-coded hosts for any subnet) or `rss` (no HTTP, no TLS; it parses typed-in XML or three built-in fake feeds). `gitlog` shows ten hard-coded commits and its parser is never called. `sqlite` is not SQLite. `cron` does not schedule or compute next-fire times. The READMEs for netscan and gitlog are honest about this; the app names are not.
2. **The seven small apps share one `main.rs`/`ui.rs` template with the same defects** (wrong arrow-key codes, unreachable Menu key, a GAM drawing API that does not exist upstream, wrong `no_std` cfg). These are listed once in the cross-cutting section and referenced per app as X1..X6.
3. **hwtools is the only app written against the real GAM API** (4 compile errors against current upstream versus 43 to 88 for the others) and the only one that touches hardware. It has no charger or battery write paths. Its two hardware writes are GPIO direction and the UART mux, and both have safety issues.

## Scores

| App | Score | Verdict |
|---|---|---|
| hwtools | 5/10 | Real API usage and sensible structure, but quit is broken, settings never load, and the UART mux and GPIO writes have side effects |
| netscan | 4/10 | Subnet math is correct; the "scan" is canned data that ignores subnet size |
| cron | 3.5/10 | Clean parser for a small subset; a built-in sample fails to parse and two advertised aliases cannot be typed |
| json | 3/10 | Parser accepts invalid numbers, has no `\u` support and no depth limit; tree view stops at depth 2 |
| regex | 3/10 | Exponential backtracking hangs the device; `[+-]` matches 51 characters; 150 lines of dead NFA code |
| sqlite | 3/10 | `INSERT` and `CREATE TABLE` are broken for multi-column tables, including the app's own sample queries |
| gitlog | 3/10 | Viewer for 10 fake commits; the parser is dead code with latent UTF-8 panics |
| rss | 2.5/10 | No feed fetching; parser panics on malformed input; no list scrolling |

---

## Cross-cutting issues (apply to json, regex, rss, sqlite, cron, gitlog, netscan)

| ID | Severity | Where | Description | Failure scenario | Fix |
|---|---|---|---|---|---|
| X1 | Critical | `app.rs` key constants in all 7 (e.g. json `app.rs:17-20`) | Arrow keys are defined as `\u{F700}`..`\u{F703}` (Apple private-use codes). The Xous keyboard service emits `'↑' '↓' '←' '→'` (U+2191/2193/2190/2192) on hardware and in hosted mode (`services/keyboard/src/main.rs:425`, `mappings/qwerty.rs:67`). | No arrow key ever matches. Lists cannot be navigated, "Left: back" never works. The toolkit's own README (line 228) says not to use these codes. hwtools uses the correct ones. | Replace the four constants with U+2190..2193. |
| X2 | High | `KEY_MENU = '\u{2234}'` in all 7 | The GAM consumes `'∴'` when it is the first key of an event and raises the main menu (`services/gam/src/contexts.rs:741-760`); it is not forwarded to the app. | "Menu: quit" and "Menu: back" never fire. Combined with X1, some states have no working exit: rss `Input` (`app.rs:102`) can only be left by a successful parse, and json `Tree` (`app.rs:162`) cannot be left at all. | Stop using Menu as an app key. Use a letter or Left for back, and do not implement in-app quit (X5). |
| X3 | High (build) | `main.rs:1-2` in all 7 | `#![cfg_attr(target_os = "xous", no_std)]` and `no_main`. Upstream apps use `target_os = "none"` (`apps/hello/src/main.rs:1-2`). On the real `riscv32imac-unknown-xous-elf` target this makes the crate `no_std`+`no_main`. | regex `engine.rs:192` uses `Box` and sqlite `sql.rs:47` uses `vec!` with no import, so both fail to compile for the device. **Suspected** for all 7: with `no_main` no `main` symbol is emitted, so the std runtime's `_start` cannot link. Hosted builds are unaffected, which is why this would not show up in hosted or lint runs. | Use the upstream attribute pair, or delete both lines. |
| X4 | High (portability) | `ui.rs` and `main.rs:57-61` pattern in all 7 | The apps call `gam.clear()`, `gam.draw_rectangle(Point, Point)`, `gam.draw_line(line)` with no canvas, `TextView::new(bounds)` with no `Gid`, `tv.text = String`, `gam::RawkeysEvent::from_message`, `xous::wait_thread(Duration)`, and never call `request_content_canvas`. None of these exist in current upstream (`RawkeysEvent` has zero hits in the tree; `wait_thread` takes a join handle). | 43 to 88 compile errors per app. I could not check git history, so this is filed as drift per the brief, but note that hwtools declares the same `xous = "0.9.69"` and uses the real canvas-based API, so this does not look like an older upstream API. Worth confirming these seven build on the working copy at all. | Port to the hwtools pattern: `request_content_canvas`, `get_canvas_bounds`, `TextView::new(gid, ...)`, `write!(tv.text, ...)`, `msg_scalar_unpack!` for raw keys, `isize` coordinates. One shared `ui` helper crate would fix all seven at once. |
| X5 | Medium | `main.rs` loop tail in all 7 | On quit the app calls `relinquish_focus`, unregisters its server and `terminate_process(0)`. Xous apps are launched once at boot and stay resident; the menu entry for a terminated app has nothing to switch to. `AppOp::Quit = 255` is never sent by anyone. | After quitting, the app cannot be reopened until reboot. Currently masked by X2. | Remove in-app quit; let the main menu switch away. |
| X6 | Low | all 7 | No unit tests despite the logic modules being pure and host-buildable (json's `CLAUDE.md` says "parser.rs is self-contained, testable without Xous"). Unused imports and constants (`KEY_RIGHT`, `Vec`, `format`) throughout. `FocusChange` only sets `needs_redraw`, so the app draws when sent to the background. READMEs (except hwtools) omit the `apps/manifest.json` entry that `APP_NAME` must match, and the `Option` token from `register_ux` is never checked. cron/gitlog/netscan/sqlite pin `num-derive 0.3` and `xous 0.9.64` while the others pin 0.4.2 and 0.9.69. | | Add `#[cfg(test)]` modules; every bug below marked **(ran)** is a ready-made test case. |

---

## precursor-json (`json-explorer`) — 3/10

A tidy recursive-descent parser that is too permissive, with a tree view that cannot reach below depth 2.

### Bugs

| Sev | file:line | Description | Failure scenario | Fix |
|---|---|---|---|---|
| Critical | `app.rs:17-23` | X1, X2. | Tree and samples cannot be navigated on device. | See X1/X2. |
| High | `app.rs:104-108`, `178-189` | `flatten` only descends while `depth < 2`, and Enter on a container just shows its summary (`"[2 items]"`). The `expanded` field is never read. | Built-in "Nested" sample: `user.roles` shows as `[2 items]` and its elements are unreachable. Any document deeper than two levels cannot be explored. | Implement expand/collapse (rebuild the flat list from a persistent tree), or flatten fully and indent. |
| High | `app.rs:173-177`, `ui.rs:94` | Down moves `cursor` but never advances `scroll`; only 15 rows are drawn. | In a tree with more than 15 nodes the selection moves off-screen and the lower nodes are never visible. | `if cursor >= scroll + 15 { scroll = cursor - 14 }`. |
| Medium | `parser.rs:108-120`, `188-231` | No nesting limit; `parse_value`/`parse_array`/`parse_object` recurse per level, and `Clone`/`Drop` of the result recurse too. **(ran)** With a 128 KiB stack (the Xous default, `kernel/src/arch/riscv/process.rs:17`) depth 400 parses and depth 500 aborts with a stack overflow on the host. Input is capped at 2000 chars, so depth 2000 is reachable. | A long run of `[` kills the process. The RV32 threshold will differ from the host figure but the cap is 4x above it. Only typed input today, so Medium; becomes High the moment input comes from a file or network. | Thread a depth counter and fail above about 64. |
| Medium | `parser.rs:152-153` | Number scanning accepts any run of digits, `.`, `e`, `E`, `+`, `-`. **(ran)** `1.2.3`, `--5`, `-e`, `1e`, `01`, `1-2+3` all parse as valid numbers. | A validator that reports invalid JSON as valid. | Implement the RFC 8259 grammar: `-? (0 \| [1-9][0-9]*) (\.[0-9]+)? ([eE][+-]?[0-9]+)?`. |
| Medium | `parser.rs:127-137` | No `\uXXXX`, `\b`, `\f`; unknown escapes are kept verbatim instead of rejected. **(ran)** `"A\b\f"` yields the literal text `A\b\f`; `"\q"` is accepted. | Any JSON with Unicode escapes displays wrong; surrogate pairs unsupported. | Decode `\u` (with surrogate pairing), add `\b`/`\f`, reject everything else. |
| Medium | `app.rs:181-189`, `parser.rs:39-46` | The Detail view shows `summary()`, which truncates strings to 27 chars. The two `if` branches are identical. | The screen whose purpose is "view a leaf value's full content" shows `"abc..."`. | Add a `full_text()` and use it for Detail. |
| Low | `parser.rs:103` | Error position is reported after `advance`, so it is off by one relative to the other messages. | | Capture `pos` before advancing. |
| Low | `app.rs:97-106` | Every flattened node stores a deep clone of its subtree (about 3x memory for a document). `Vec<char>` input costs 4 bytes per char. | Heap pressure on a small device. | Store indices or references; parse over bytes. |

### Upstream drift
53 errors, all X4 plus `xous::String::<128>::from_str` (now `String::from`). No logic changes needed beyond the port.

### Improvements
1. Fix X1 to X3, then scrolling and real expand/collapse.
2. Strict number and escape handling plus a depth limit, with unit tests (the parser is already host-testable).
3. There is no practical way to enter JSON: no paste, no newline, no cursor movement, 2000 chars by thumb. Load from a PDDB key or offer a modal text entry.
4. Show the key path of the selected node; show duplicate-key warnings.

---

## precursor-regex (`regex-tester`) — 3/10

The matcher is a small hand-written backtracker with an exponential worst case and a character-class bug, sitting beside an unused, unfinished NFA compiler.

### Bugs

| Sev | file:line | Description | Failure scenario | Fix |
|---|---|---|---|---|
| Critical | `app.rs:16-22` | X1, X2. | Cannot switch fields with Down, open the cheat sheet, or scroll results. | See X1/X2. |
| High | `engine.rs:259-292`, `316-341` | Unmemoised recursive backtracking, restarted at every text offset, run synchronously on the UI thread. **(ran)** Pattern `a*a*…b` against 30 `a`s on an M-series host: 5 groups 0.1 s, 6 groups 1.3 s, 8 groups (17 chars) 35 s. Growth is roughly 12x per added group; the app allows 120-char patterns and text. | Typing `a*a*a*a*a*a*a*a*b` and a line of `a`s freezes the app for hours on a 100 MHz CPU. There is no cancel; the only recovery is a reboot. | Memoise failed `(pat_pos, text_pos)` pairs (120x120 bits), which makes it O(P·T), or add a step budget that returns "pattern too complex". |
| High | `engine.rs:210-220` (also `81-91`) | Range detection checks `pattern[j+1] == '-'` and `j + 2 < len` but not that `pattern[j+2]` is `]`. **(ran)** `[+-]` on `a+Z9-]` matches `+`, `Z`, `9`, `-`, `]` (the range becomes `+`..`]` and swallows the bracket). `[a-]` matches nothing. | The common sign class `[+-]?\d+` matches letters and digits. | Treat `-` as a literal when the next char is `]`. |
| Medium | `engine.rs:1-4`, `133-135`, `243` | The header claims `\|`, `()`, `{n}`, `{n,m}` and "NFA with Thompson construction". The matcher treats all of them as literals, while `validate` (`360-364`) still balances parentheses as if they grouped. **(ran)** `(ab)+` does not match `abab` but does match `(ab))`; `a\|b` matches only the literal `a|b`; `a{2}` matches only `a{2}`. | A valid-looking pattern silently means something else; the tool teaches wrong regex semantics. The README and cheat sheet list only the supported subset, so the doc comment is the wrong part. | Either implement groups and alternation or make `validate` reject `( ) \| { }` with "not supported". |
| Medium | `engine.rs:46-168` | `compile`, `Nfa`, `Fragment`, `NfaState`, `state_matches` are never called. `*`/`+`/`?` push `Split(_, 0)` placeholders that are never patched, so it could not work if it were called. | About 150 lines of dead, misleading code and compiler warnings. | Delete, or finish it and drop the backtracker (which would also fix the blow-up). |
| Low | `engine.rs:328-337` | Zero-length matches are discarded. **(ran)** `^$` on empty text and `a*` on `bbb` report "No matches found". | | Report empty matches as `[n..n] ""`. |
| Low | `engine.rs:365-367` | `validate` only rejects a quantifier at index 0. **(ran)** `a**` is accepted and the second `*` is a literal. `[\d]` matches `\` or `d`. `engine.rs:180-189` computes an unused `quantifier`. | | Track "previous token is quantifiable"; support escapes inside classes. |
| Low | `app.rs:150` | `scroll` grows without bound in Results. | Scrolling past the end shows a blank list. | Clamp to `matches.len() - visible`. |
| Low | `engine.rs:192-243` | A `Box<dyn Fn>` and a `Vec` are heap-allocated on every recursion step. | Slow and allocator-heavy even on benign patterns. | Use an enum matcher. |

### Upstream drift
43 errors, all X4. X3 additionally breaks device compilation (`Box` unimported under `no_std`).

### Improvements
1. Memoise or budget the matcher before anything else; a regex tester that can brick the session on plausible input is the top risk in this slice.
2. Fix the class parser and make unsupported syntax an error.
3. Show the match in context (invert the matched span in the text) rather than index pairs.
4. Unit tests: the **(ran)** cases above.

---

## precursor-rss (`rss-reader`) — 2.5/10

Not a reader: there is no network code. It string-searches typed XML, panics on mismatched tags, and cannot scroll.

### Network / TLS
None. `Cargo.toml` has no `net`, `tls` or `dns` dependency and no code path fetches anything. Input is three hard-coded sample feeds or up to 4000 hand-typed characters. The sample headlines are invented ("Rust Hits 2.0", "element 120 created at CERN") and shown under realistic feed names; they are prefixed "Sample:" on the list screen only. If fetching is added later, the parser issues below become remotely triggerable.

### Bugs

| Sev | file:line | Description | Failure scenario | Fix |
|---|---|---|---|---|
| Critical | `app.rs:10-16` | X1, X2. Additionally `Input` state has no exit other than Menu (`app.rs:102`). | Cannot browse samples or items; stuck in the input screen unless the typed XML parses. | See X1/X2. |
| High | `rss.rs:116-121` | `<item` start offsets and `</item>` end offsets are paired by index with no ordering check, then sliced `&xml[start..end]`. **(ran)** Input `</item><item>` panics (start 7 > end 0). | Any feed with a stray close tag, or `<item` appearing after its paired `</item>`, crashes the app. | Scan sequentially: find `<item`, then the next `</item>` after it. |
| Medium | `rss.rs:28-52`, `110` | `extract_tag` matches the prefix `<tag` and takes the first hit anywhere. **(ran)** A channel with `<image><title>Logo</title></image>` before its own title yields feed title `Logo`; a channel with no title takes the first item's title; `<items>` counts as an `<item>` start; with a missing `</item>` two items merge into one. | Wrong titles and merged or dropped items on real feeds. | Require `>`, space or `/` after the tag name; extract channel fields from the region before the first `<item`. |
| Medium | `app.rs:63-65`, `ui.rs:61-67` | Parse errors are stored but only drawn on the Samples screen. | Pressing Enter on bad XML in the Input screen does nothing visible. | Draw `app.error` in `draw_input`, clear it on edit. |
| Medium | `ui.rs:88-90`, `app.rs:124-128` | Feed list draws `take(9)` with no scroll offset; the cursor can move to item 10 and beyond. | Selection disappears; items past the ninth can be opened but never seen in the list. | Add a scroll offset as gitlog does. |
| Medium | `app.rs:142-143`, `ui.rs:124-127` | Article `scroll` is updated but never used when drawing. `link` is parsed and never shown. | Long descriptions are cut off at the bottom of the screen with no way to read on. | Paginate the description; show the link. |
| Medium | `rss.rs:41`, `47`, `83-95` | CDATA is only recognised with no surrounding whitespace, entities are decoded inside CDATA, and decoding happens before tag stripping. **(ran)** ` <![CDATA[<p>Hello</p> 1 > 0]]> ` becomes ` Hello 1  0]] `; `a &lt; b and c &gt; d` becomes `a  d`. | Stray `]]` and silently deleted text. | Trim before the CDATA test; do not decode CDATA; strip tags before decoding entities. |
| Low | `rss.rs:55-80` | Only five named entities. **(ran)** `&#8217;` and `&nbsp;` pass through raw; numeric entities are everywhere in real feeds. Line 61 rebuilds the remaining string at every `&`, which is O(n²) (88 ms on host for 4000 `&`, so seconds on device). | | Decode `&#N;`/`&#xN;`; compare against the char slice instead of allocating. |
| Low | `rss.rs:137`, `ui.rs:114-129` | **(ran)** `<item>` with no `</item>` returns `Ok` with zero items; opening an article with no current item draws a blank screen with no header or footer. | | Return an error for zero items; draw an empty-state message. |

### Upstream drift
51 errors, all X4.

### Improvements
1. Decide what this app is. A reader needs `net`/TLS and a feed-URL list in PDDB; otherwise rename it "RSS parser demo".
2. Replace substring search with a small forward-only tag scanner; it fixes the panic, the prefix matching and the title confusion together.
3. Scrolling in both list and article views.
4. Label sample content as fictional in the article view as well.

---

## precursor-sqlite (`sqlite`) — 3/10

A toy in-memory table filter. Two of its own ten sample queries do not work, and comparisons on the sample `price` column are wrong.

### Bugs

| Sev | file:line | Description | Failure scenario | Fix |
|---|---|---|---|---|
| Critical | `app.rs:9-15` | X1, X2. | Samples, history and result scrolling unreachable on device. | See X1/X2. |
| High | `sql.rs:333`, `210-212` | The tokenizer discards commas and parentheses; `exec_insert` then re-joins tokens with spaces and splits on `,`. **(ran)** The built-in sample `INSERT INTO users VALUES (6, 'Frank', 'frank@example.com', 29)` returns `Error: Expected 4 values, got 1`. | INSERT works only on single-column tables. | Use the tokens after `VALUES` directly as the value list. |
| High | `sql.rs:237-242` | Same cause in `exec_create`. **(ran)** The built-in sample `CREATE TABLE notes (id, title, body)` reports success and creates a table with one column (`DESCRIBE notes` shows only `id`). | Silent wrong schema; every later INSERT fails. | Use `tokens[3..]` as column names (skipping type words explicitly if types are to be allowed). |
| Medium | `sql.rs:241` | `parts[0]` on an empty split. **(ran)** `CREATE TABLE t ''` panics. | App crash from a typed query. | Skip empty names and return an error. |
| Medium | `sql.rs:344-365` | Ordering comparisons parse `i64` only and otherwise fall back to string comparison. **(ran)** `SELECT name FROM products WHERE price < 5` returns Gadget (24.99), Doohickey (4.99) and Thingamajig (14.99). | Wrong answers on the sample data's decimal column. `=` is also purely textual (`1` vs `1.0`, case-sensitive). | Parse decimals as fixed-point integers; compare numerically when both sides parse. |
| Medium | `sql.rs:136-198` | Anything after the first `WHERE col op val` is ignored. **(ran)** `... WHERE age > 28 AND name = 'Bob'` returns Alice and Carol; `SELECT name FROM users ORDER BY age LIMIT 1` returns four rows (one fewer than expected only because an earlier sample deleted Eve). | Silent wrong results from ordinary SQL in a "learning tool". | Reject trailing tokens with "not supported: AND/ORDER BY/LIMIT". |
| Medium | `sql.rs:169`, `314-342` | Operators are not tokenised. **(ran)** `WHERE age>28` gives "Incomplete WHERE clause". Quoted strings lose their quotes, so `'from'` or `'where'` as a value can be mistaken for a keyword. | Queries typed without spaces fail with a misleading message. | Tokenise operators; keep a token-kind tag for quoted strings. |
| Low | `ui.rs:92-97`, `113-118` | Columns are padded to 12 chars with spaces but drawn in proportional `Small`/`Bold` fonts, and four columns (51 chars) exceed the line width. | The README's "column-aligned output" does not align; rows wrap or clip. | Use `GlyphStyle::Monospace` or draw each cell at a fixed x. |
| Low | `app.rs:52`, `92`; `ui.rs:126` | History grows without bound and stores duplicates; `result_scroll` is unbounded (indicator shows e.g. `40-25/25`); the table list in `draw_input` runs under the footer after about 20 tables. | | Cap history at about 20, clamp scroll. |

### Upstream drift
48 errors, all X4. X3 additionally breaks device compilation (`vec!` unimported under `no_std`).

### Improvements
1. Rename: the package `sqlite` and title "SQLite Client" promise a database engine. "SQL Explorer" (already the `APP_NAME`) is accurate.
2. Fix INSERT/CREATE and add tests that execute every entry in `SAMPLE_QUERIES`; that test alone would have caught both High bugs.
3. Reject unsupported clauses instead of ignoring them.
4. Optional PDDB persistence; `UPDATE` and `DROP TABLE`.

---

## precursor-cron — 3.5/10

A correct parser for a deliberately tiny subset (`*`, `N`, `A-B`, `*/N`), wrapped in a UI that blocks its own aliases. It is an expression explainer, not a scheduler.

### Scope note
There is no next-fire computation, no clock access, and no day-of-month/day-of-week logic at all, so there are no bugs in those areas; they are simply missing. The name "Cron Scheduler" overstates it.

### Bugs

| Sev | file:line | Description | Failure scenario | Fix |
|---|---|---|---|---|
| Critical | `app.rs:9-15` | X1, X2. | Sample browser and reference cannot be navigated. | See X1/X2. |
| High | `app.rs:64-65` | In the input state `s`/`S` and `r`/`R` open Samples and Reference before the character can be typed. | The advertised aliases `@hourly` and `@yearly` cannot be entered (both contain `r`), nor could names such as `sun` or `fri` if added. | Bind these to keys outside the input alphabet, or only when the buffer is empty. |
| High | `cron.rs:37-60`, `193` | No list support. **(ran)** The built-in sample "Weekends at noon" (`0 12 * * 0,6`) fails with "Invalid number in expression". | One of ten shipped samples is an error. | Parse comma lists into a `Vec` of items (or a 64-bit mask per field). |
| Medium | `cron.rs:109-147` | Descriptions do not combine minute and hour. **(ran)** `30 9 * * *` is described as "at minute 30, at 9:00"; `*/15 9 * * *` as "every 15 min, at 9:00"; `0 0 13 * 5` as "on day 13, on Fri". | Reads as 9:00 rather than 9:30. The last one implies AND, whereas cron fires when either day field matches if both are restricted. `0 0 31 2 *` (never fires) is described without warning. | Format `HH:MM` when both are values; say "or" for restricted DOM+DOW; warn on impossible dates. |
| Medium | `cron.rs:87`, `37-60` | **(ran)** Day-of-week `7` (Sunday in most crons) is rejected; `1-5/2`, `0/15`, and names (`JAN`, `MON`) are rejected. | Common real-world expressions report errors, so the tool cannot be used to check a crontab line. | Accept 7 as Sunday, `range/step`, and three-letter names. |
| Low | `cron.rs:43-57` | `u8::from_str` accepts a leading `+`. **(ran)** `+5 * * * *` parses as minute 5. | | Reject non-digit input explicitly. |
| Low | `app.rs:105` | Reference scroll allows 12 steps but only 4 are needed (30 lines, 26 visible). | Blank space at the bottom. | Clamp to `lines.len() - visible`. |
| Low | `ui.rs:100` | `{:8}` padding in a proportional font does not align the field table. | | Fixed x positions or monospace. |

### Upstream drift
44 errors, all X4.

### Improvements
1. Lists, range/step, names and DOW 7; then the description logic. Represent each field as a bitmask, which makes all of this simple and enables the next item.
2. Add "next 5 fire times" using the RTC; that is the feature the name promises, and where DOM/DOW OR semantics and month-length handling need tests.
3. Unit-test every sample expression.

---

## precursor-gitlog — 3/10

A list/detail/search UI over ten fabricated commits. The git-log parser exists but nothing calls it and there is no way to load data.

### Bugs

| Sev | file:line | Description | Failure scenario | Fix |
|---|---|---|---|---|
| Critical | `app.rs:9-15` | X1, X2. | Cannot scroll the list or go back with Left (Backspace does work in detail view). | See X1/X2. |
| High | `gitlog.rs:59-93`; `app.rs:34` | `parse_git_log` is dead code; `GitLogApp::new` always loads `sample_commits()`. The `error` field is unused. | The README feature "parses standard `git log` output" is unreachable; the app can only ever show its demo data. | Add an input path (PDDB key, or a USB/serial import), or describe the app as a demo. |
| Medium (latent) | `gitlog.rs:27`, `34`, `54` | `short_hash`, `summary` and `short_date` slice by byte index. **(ran)** A hash with a multi-byte char at byte 6 panics in `short_hash`; `summary` panics whenever byte 30 of the message falls inside a multi-byte char. | Harmless with the ASCII samples; a crash on the first real log with a non-ASCII subject or author once the parser is wired up. | Use `chars().take(n)` or `char_indices`. |
| Medium (latent) | `gitlog.rs:68-81` | Header detection runs on the trimmed line. **(ran)** A message body line `    commit to the new approach` starts a new bogus commit; `    Date: was wrong before` overwrites the date. Ref decorations stay in the hash (`abc1234def (HEAD -> main)`). | Corrupted commit list from ordinary logs. | Match headers only at column 0; split the hash at the first space. |
| Low | `app.rs:104-105`; `ui.rs:153` | `detail_scroll` changes but is never used. | Long messages are clipped. | Paginate. |
| Low | `app.rs:116` | Search buffer has no length limit and is echoed into the header. | Header overflow. | Cap at about 24 chars. |
| Low | `gitlog.rs:98-106` | Sample data embeds a real email address alongside invented contributors. | Cosmetic; worth a conscious choice. | Use `example.com` throughout. |

### Upstream drift
88 errors (the most in the slice because `ui.rs` builds every `TextView` inline), all X4.

### Improvements
1. Give it a data source or rename it.
2. Fix the two latent parser bugs and add tests before wiring it up; the **(ran)** input is in the harness.
3. Up/Down in the detail view should step to the adjacent commit.

---

## precursor-netscan — 4/10

A correct subnet calculator and port table with a "scan" screen that fabricates results.

### What it actually scans
Nothing. There are no sockets, no `net` dependency, no timers and no blocking calls; `simulate_scan` returns six fixed hosts offset from the network address. There is therefore no legal or network-etiquette exposure and no timeout or UI-blocking issue. The risk runs the other way: output that looks like scan results for whatever range the user typed.

### Bugs

| Sev | file:line | Description | Failure scenario | Fix |
|---|---|---|---|---|
| Critical | `app.rs:9-15` | X1, X2. | Right (ports, scan) and list navigation do not work; `S` still starts the scan. | See X1/X2. |
| Medium | `scanner.rs:161-203` | The fake hosts are `network + {1,2,10,20,50,100}` regardless of prefix length. **(ran)** `10.0.0.5/30` lists `10.0.0.14`, `.24`, `.54`, `.104`, all outside the subnet. `8.8.8.8` (classful /8) shows `gateway.local` at `8.0.0.1`. | "Results" that contradict the subnet calculation shown one screen earlier, and invented hosts on public address space. | Clamp to the host range, and refuse to simulate outside RFC 1918 space. |
| Medium | `ui.rs:150`, `188-216` | Only the list header says "(Simulated)". The host-detail screen shows IP, hostname and open ports with no such label. | A screenshot or a glance at the detail screen reads as a real finding. | Put "SIMULATED" on every scan screen; consider renaming the app "Subnet Calc". |
| Low | `scanner.rs:167` | `base + 1` overflows for `255.255.255.255/32`. **(ran)** Panics with overflow checks on (debug/hosted builds); wraps to `0.0.0.0` in release. | Crash in hosted testing. | `wrapping_add` or skip when the range is exhausted. |
| Low | `ui.rs:120-139` | 20 rows at 22 px starting at y=60 end at y=500, past the footer line at 490 and over the position indicator at 474. `app.rs:105` lets `port_scroll` reach 24, leaving one row on screen. | Overlapping text on the port table. | 18 visible rows; clamp scroll to `len - visible`. |
| Low | `scanner.rs:65-71`, `126-151` | Class is reported for 0.x and 127.x as "A" with no loopback/private/link-local note. Simulated ports 139, 631, 953, 9100 are not in the port table and show as "Unknown". **(ran)** `010.1.1.1` is accepted as 10.1.1.1. | | Add the missing ports; annotate special ranges; reject leading zeros. |

### Upstream drift
50 errors, all X4.

### Improvements
1. If real scanning is ever added: default to the device's own subnet only, require explicit confirmation for anything else, cap the range (for example /24), use short connect timeouts on a worker thread with a cancel key, and never preload a target. The current default input of `192.168.1.0/24` is fine for a calculator but would be the wrong default for a live scanner.
2. Useful calculator additions: wildcard mask, binary view, subnet splitting, "is address X in this subnet".
3. Unit tests for `subnet_calc` edge prefixes (0, 30, 31, 32), which all pass today **(ran)**.

---

## precursor-hwtools — 5/10

The best-built app in the slice and the only one with hardware side effects. It reads battery, EC and ADC data correctly through `com` and `llio`. It has no charger, boost, or battery-register writes. Its problems are a quit path that half-kills the app, settings that never load, and two hardware writes (GPIO drive, UART mux) applied without the user asking.

### Hardware access inventory
- **Reads:** `get_batt_stats_blocking`, `is_charging`, `get_ec_uptime`, `get_ec_git_rev`, `get_ec_sw_tag`, `gyro_read_blocking` (COM); `soc_gitrev`, `soc_dna`, `adc_vbus`, `adc_temperature`, `adc_vccint`, `adc_vccaux`, `adc_gpio2`, `adc_gpio5` (LLIO). All exist upstream with the signatures used.
- **Writes:** `llio.gpio_data_direction(mask)` (`gpio_tab.rs:71`) and `llio.set_uart_mux(..)` (`uart_tab.rs:59`, `288`). Nothing else.

### Bugs

| Sev | file:line | Description | Failure scenario | Fix |
|---|---|---|---|---|
| High | `main.rs:84-99` | When `handle_key` returns true (`q`), the code sends `PumpOp::Quit`, disconnects from the pump, and `break`s. That `break` is inside `for &key in keys.iter()`, so it leaves the key loop, not the main `loop`. | Pressing `q` kills the pump thread and leaves the app running with frozen data and no further refreshes, permanently. `save_settings()` (`main.rs:141`) is never reached, because `AppOp::Quit` is never sent by anything. | Use a `quit` flag and break the outer loop after the match, or drop in-app quit (X5) and save settings on change and on background. |
| High | `storage.rs:36` vs `60-66` | `load` requires `buf.len() >= 10`; `save` writes 9 bytes (5 + 4). | Even when a save does happen, every load logs "Settings data too short" and returns defaults. Persistence, a README feature, has never worked. | Compare against 9, and add a leading version byte while changing the format. |
| High (genuine compile error) | `storage.rs:33`, `68` | `match pddb.get(..) { .. }` is the tail expression of both functions, so the `PddbKey<'_>` temporary outlives the local `pddb` (E0597). This is a Rust 2021 borrow rule, not an API change. | Does not compile against any xous-core where `PddbKey` borrows the `Pddb`. Worth checking whether the GitHub copy lags the working copy. | Bind the result to a `let` or end the `match` with `;`. |
| High (safety) | `uart_tab.rs:59`, `288`; upstream `services/llio/src/llio_hw.rs:261-285` | `UartTab::new` calls `set_uart_mux` at app launch regardless of the visible tab, and `m` cycles it. Upstream: selecting Kernel or Application sets `POWER_DISABLE_WFI` ("disabling WFI so that kernel console works"), and selecting Log clears it with the warning "unsafe re-enabling WFI -- if you issued this command at a bad time, could have side effects" (it names SPINOR flashing). The mux is never restored. | Merely launching HW Tools clears the WFI override. Cycling to Kernel/App leaves WFI disabled after the app is backgrounded, which drains the battery, and routes the kernel console to the debug connector on a security device. Once the settings bug is fixed, a saved Kernel/App choice would be reapplied silently at every boot-time launch. | Do not touch the mux in `new`. Change it only on an explicit keypress with a confirmation line, show the power consequence, restore Log on background, and do not persist it. |
| Medium (safety) | `gpio_tab.rs:63-71`, `257-279` | Direction is pushed to hardware inside `refresh()` on every 2 s pump, from state that is restored from PDDB. Pressing a digit on an input pin switches it to output immediately (`265-268`). The app never writes the output register, so an "output" pin drives whatever `gpio_dout` holds (0 at reset). Pins are left as outputs when the tab or app is left. | A pin wired to something that drives it high gets driven low against it, with no confirmation, and stays that way after leaving the app. Today the load bug masks the restore path; fixing persistence makes saved outputs re-drive at launch. | Apply direction only on the keypress, require confirmation to enable output, return all pins to input on background/exit, and do not restore output direction from storage. |
| Medium | `gpio_tab.rs:152-156`, `260-263`, `283-289` | `outputs[]` is UI-only state; toggling logs "GPIO3 output toggled to true" and the table shows `[1]`, but nothing is written to hardware. | The display says a pin is high while it is driving low. The small note at the bottom says toggling is unsupported, but the table contradicts it. | Remove the Out column and the toggle keys until `gpio_dout` is reachable. `Opcode::GpioDataOut`/`GpioDataIn` exist in the llio server (`api.rs:59-60`) with no `llio_lib` wrapper, so a small upstream PR would unlock this. |
| Medium | `app.rs:252-264` | Global keys `1`/`2`/`3`/`q` are handled before the tab's own handler. | In the GPIO tab, pins 1, 2 and 3 cannot be selected or toggled by number (pin 2 is one of the two ADC pins). In the UART tab those characters cannot be typed into the TX buffer. | Make tab switching arrows-only, or give the active tab first refusal. |
| Medium | `main.rs:75-102` | The Rawkeys arm never redraws; only `switch_tab` does. | Cursor movement, direction changes, pause, mux and typed TX text appear up to 2 s late (next pump), and never after `q`. `r` on the System tab refreshes data without redrawing. | Call `app.redraw()` after handling keys. |
| Medium | `main.rs:86-97`, `170-181` | Main sends a *blocking* `PumpOp::Quit` to the pump thread, while the pump thread sends a *blocking* `AppOp::Pump` to main and sleeps 2 s inside its handler. | If the pump's blocking send is issued while the `q` key message is still queued in main, main then blocks on the pump and the pump is blocked on main: deadlock. Otherwise quit stalls the UI for up to 2 s. Narrow window; by reasoning, not reproduced. | Send Quit non-blocking, or have the pump use a non-blocking send to main. |
| Low | `uart_tab.rs:71-83`, `147-151`, `309-315` | The UART monitor captures nothing: the log holds one placeholder line, status reads "Capturing", and Enter on TX only writes the text to `log::info!` and the on-screen list. | Reads as a working serial terminal. The README's limitations table admits RX is a placeholder; TX is described as "for sending data". | Label the tab "UART mux" and remove TX until it is real. |
| Low | `gpio_tab.rs:81-87`, `172` | Comment says the digital threshold is "~1.5V = ~1200 raw", but the app's own conversion (`raw * 3300 / 4096`) puts 1200 at 0.97 V. **Suspected:** the 3.3 V full-scale factor itself needs checking against the XADC input range and board dividers. | Inferred logic levels flip at the wrong voltage. | Derive the threshold from the conversion; verify the scale on hardware. |
| Low | `ui.rs:97` | `percent` is not clamped. | A gas-gauge reading above 100 draws the fill outside the bar. | `percent.min(100)`. |
| Low | `app.rs:229`; `uart_tab.rs:87`, `197`; `system_tab.rs:59-68` | Help text says "F2:Refresh" (the key is `r`). A new `Ticktimer` connection is made per log line. Timestamps print `[ss:cs]`, which reads as `mm:ss`. Static values (EC git rev, SoC rev, DNA) are re-fetched every 2 s, about 12 blocking IPC round-trips per refresh on the UI thread. `auto_refresh` and `refresh_interval_ms` are stored but unused. | | Cache static values once; fix labels. |
| Low | `system_tab.rs:177-179` | The SoC DNA (unique device identifier) is shown in full. | Privacy leak in shared screenshots. | Mask by default. |

### Upstream drift
Four errors: the two `storage.rs` borrow errors above (genuine) and `xous::destroy_server` now being `unsafe` (`main.rs:144`, `196`). One unused-import warning. Otherwise this app tracks upstream.

### Improvements
1. Fix quit/save/load together: save on change, load with a versioned 9-byte check, drop `q`.
2. Make both hardware writes explicit, confirmed, and reverted on background.
3. Redraw on keypress.
4. Either implement the UART tab or reduce it to a mux selector with the WFI warning.
5. Upstream `gpio_dout`/`gpio_din` wrappers in `llio_lib`, then finish the GPIO tab.

---

## Cross-cutting summary

1. **Template defects (X1 to X6)** make all seven small apps non-navigable on real hardware and non-buildable for the device target, independently of upstream drift. Fixing the shared `main.rs`/`ui.rs` template once, ideally as a small `precursor-app-kit` crate modelled on hwtools, repairs all seven.
2. **Shipped samples are untested.** sqlite's INSERT and CREATE samples and cron's "Weekends at noon" fail on first use. A test that runs every built-in sample would have caught all three.
3. **Scroll handling is copied inconsistently.** gitlog and netscan do it right; json, rss, regex results, sqlite results and cron reference either never scroll or never clamp.
4. **Byte-index slicing and unchecked indexing on input** (`rss.rs:121`, `sql.rs:241`, `gitlog.rs:27/34/54`) are safe only because input is ASCII typed on the keyboard. Each becomes a crash once any external data source is added.
5. **No resource limits in the hand-written parsers:** JSON recursion depth, regex backtracking steps, RSS quadratic entity decoding. All run on the UI thread with no cancel.
6. **Names promise more than the code does** (SQLite Client, Network Scanner, RSS Reader, Cron Scheduler, Git Log Viewer). Either rename to "explorer/demo" or build the missing half; for netscan and rss that half is where the real security design work lies.
7. **Proportional-font column padding** (sqlite, cron, netscan port table) does not align; use `GlyphStyle::Monospace` as hwtools does.
8. **Dependency pins differ within the slice** (`num-derive` 0.3 vs 0.4.2, `xous` 0.9.64 vs 0.9.69).
