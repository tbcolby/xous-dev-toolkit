# Productivity slice: calc, writer, timers

Scope: `precursor-calc` (pkg `calc`), `precursor-writer` (pkg `writer` + `writer-core`), `precursor-timers` (pkg `timers` + `timer-core`). Every `.rs` file in the three repos was read. Paths below are relative to each repo root.

How findings were checked:

- **Host-run**: reproduced by running the code on the host. `writer-core` and `timer-core` were copied to `SCRATCH/hosttest/` and tested there; calc's pure modules (`functions`, `algebraic`, `rpn`, `display`, `memory`) were copied into a scratch lib crate `SCRATCH/hosttest/calc`. Source repos were not modified.
- **Read**: verified by reading the app code and the upstream xous-core code it calls (keyboard maps, PDDB, GAM, ticktimer).
- **Suspected**: plausible but not confirmed; needs a device or Renode run.

Nothing was run on hardware or in Renode.

## Host test results

| Crate | Existing tests | Result |
|---|---|---|
| `writer-core` | 40 | all pass |
| `timer-core` | 7 | all pass |
| calc pure modules (extracted) | 17 | all pass |

The existing tests pass but miss every bug below: they cover only ASCII input, single-operation RPN and X-register-only assertions. Probe tests I added are in `SCRATCH/hosttest/*/tests/probe.rs` and `SCRATCH/hosttest/calc/src/lib.rs`.

The calc tests cannot run in place because calc is a single bin crate with xous dependencies. Its README does not say how to run them.

## Upstream drift (all three)

Each app has exactly one compile error against xous-core HEAD 31fbe23, and it is the same one: `xous::destroy_server(sid)` is now `unsafe` (E0133).

- `calc/src/main.rs:145`
- `writer/src/main.rs:1285`
- `timers/src/main.rs:1209`

Fix: wrap in `unsafe { ... }`. No other API mismatches: no `graphics_server`, `RawkeysEvent`, `set_len` or arg-count errors in this slice. All three already have entries in the integrated tree's `apps/manifest.json` with matching `context_name`.

---

# calc

**Verdict:** clean structure and a decent function library, but both entry modes give wrong answers for common inputs and the app exits on any unmapped key. **Health: 4/10.**

## Bugs

| Sev | Location | Description | Failure scenario | Fix | Check |
|---|---|---|---|---|---|
| Critical | `src/app.rs:264`, `src/app.rs:137`, `src/main.rs:105-109` | `handle_action` returns `false` for `KeyAction::None`, and `main` treats `false` as quit. Any unmapped key terminates the process. | Press an arrow key, `,`, `?`, uppercase `Z`, any unmapped lowercase letter in RPN mode, or any non-digit inside an F-menu. The calculator saves and calls `terminate_process`. | Return `true` for `None`/`MenuSelect`; return `false` only for `Quit`. Better: return an enum. | Read |
| Critical | `src/keymap.rs:192-195` | F1-F4 are matched as `U+F704..F707`. The Precursor keyboard sends `U+0011..U+0014` (`services/keyboard/src/mappings/qwerty.rs:60-64`; writer and timers use the right codes). | On hardware, pressing F1 hits the unmapped-key path above and quits the app. The MATH/TRIG/MODE/MEM menus are unreachable. | Match `'\u{0011}'..='\u{0014}'`; keep the PUA codes as hosted-mode aliases. | Read |
| Critical | `src/rpn.rs:145-157`, `src/rpn.rs:160-171` | Enter does not disable stack lift. `enter()` duplicates X into Y, then the next digit lifts again. | `1 Enter 2 Enter 3 + +` gives **7** (stack `[7,1,1,1]`); correct is 6. `2 Enter 3 *` leaves a stray 2 in Y. Any chained RPN calculation is wrong. | Add a `lift_enabled` flag: Enter sets it false, operations set it true, digit entry lifts only when true. Add multi-step tests asserting the whole stack. | Host-run |
| High | `src/algebraic.rs:201`, `src/algebraic.rs:214-217` | Prefix functions (including unary `Negate`) are pushed on the operator stack and never popped by a following operator, so they apply to the entire rest of the expression. | `-ans+1` with ans=5 gives **-6** (should be -4). `-pi+1` gives -4.14. `-sqrt(4)+1` gives **-3**. `sin 30+1` gives sin(31). | Give unary minus a real precedence (above `*`, below `^`). Require or auto-insert parentheses for named functions, or pop functions when an operator arrives. | Host-run |
| High | `src/algebraic.rs:45-47` | A unary minus before a digit is folded into the literal. | `-2^2` gives **4**, while `-ans^2` gives -25. The two paths disagree. | Always emit `Negate` and let precedence resolve it. | Host-run |
| High | `src/app.rs:330-338`, `src/functions.rs:192-199` | In ALG mode a function key inserts `func.name()` plus `(`. For Square, Cube, Reciprocal and Factorial the names are `x²`, `x³`, `1/x`, `!`, which the tokenizer cannot parse. `Func::from_name` also has no entry for them. | Pressing Square in ALG inserts `x²(`; Enter shows `ERR: PARSE`. Factorial is unusable in ALG mode. | Insert parseable text (`^2`, `^3`, `1/(`, `fact(`) and add `fact`, `neg`, `exp10` to `from_name`. Add a postfix `!` token. | Host-run |
| High | `src/storage.rs:96-108` | Save writes JSON at offset 0 with no truncation. PDDB writes never truncate (`services/pddb/src/main.rs:2271-2279` passes `truncate=false`). JSON length varies with the values. | ans = 3.141592653589793 is saved, then ans = 5 is saved. The key now holds the short JSON plus the old tail. On next boot `serde_json::from_slice` fails and `load` silently returns defaults: all ten memory registers, mode and ans are lost. | `delete_key` before writing (vault's pattern), or use a fixed-size binary record with a version byte. | Read |
| Medium | `src/storage.rs:91`, `src/functions.rs:458-460` | `+`, `-`, `*`, Square, Cube, sinh and cosh do not check overflow, so ans can become infinite or NaN. `serde_json` writes non-finite floats as `null`, which fails to load as `f64`. | `1e308*10` shows `∞`. Backgrounding the app saves `"ans":null`; next boot all settings reset. | Return `CalcError::Overflow` for any non-finite result in `Op::evaluate` and `Func::evaluate`; sanitise before saving. | Host-run (overflow); Read (persistence) |
| Medium | `src/keymap.rs:92`, `src/keymap.rs:211-250` | The shift layer waits for `U+000F`. The keyboard service never sends that; Shift only changes which character a key emits, and Shift+digit emits the digit. | Every "Shift+key" binding in the README (sin, cos, tan, ln, log, sqrt, pi, change sign) is unreachable on hardware. With F-keys also broken, the only way to enter functions is typing names in ALG. RPN has no way to change sign or apply any function. | Use a prefix key that exists on the keyboard, or the F-key menus once fixed. | Read |
| Medium | `src/rpn.rs:128-133` | `clear_x` does not disable stack lift. | `7 Enter 9 CLx 4` gives stack `[4,0,7,7]`; correct is `[4,7,...]`. | Same `lift_enabled` flag as the Enter fix. | Host-run |
| Medium | `src/display.rs:48` | Fixed formatting uses `{:.10}` regardless of magnitude, so it prints more digits than an f64 holds. | `12345678.9` displays as `12345678.9000000004`; `1234567890.123` as `1234567890.1229999065`. | Format to 12-14 significant digits total, then trim zeros. | Host-run |
| Medium | `src/functions.rs:219-220` | Degree-mode trig converts to radians first, leaving float residue at exact angles. | `sin(180)` displays `1.2246468e-16`; `cos(90)` displays `6.1232340e-17`. | In DEG/GRAD, reduce the angle modulo 360/400 and special-case quadrant multiples, or snap results below about 1e-15 to 0. | Host-run |
| Medium | `src/ui.rs:7-8`, `src/ui.rs:344-345`, `src/ui.rs:273` | Layout is hard-coded to 336x536, but the app registers `UxType::Chat`, whose canvas is shorter. `screensize` is fetched in `main.rs:70` and never used. Stack labels get a 20px-wide box. | In `screenshots/09_rpn_mode.png` the F1-F4 menu bar is not visible, the `T:/Z:/Y:/X:` labels render as empty boxes, and stack digits are clipped at the bottom. | Lay out from `get_canvas_bounds`. Use `UxType::Framebuffer` (as `apps/ball` does) to get the full area. Widen label boxes and raise row height to at least 18px. | Read (screenshot) |
| Medium | `src/storage.rs:60` | `Pddb::try_mount()` runs in the constructor on the main thread at process start. Upstream docs say it can raise the password prompt. | Suspected: at boot the calculator competes with the normal unlock flow, or `load` runs before the PDDB is mounted and settings silently fall back to defaults. | Load lazily on first foreground after `is_mounted_nonblocking()`, or wait on `is_mounted_blocking()` in a helper thread as vault does. | Suspected |
| Low | `src/display.rs:76-82`, `src/display.rs:100-101` | `-int_val` and `.abs()` overflow for `i64::MIN`. HEX/OCT/BIN also silently truncate fractions and saturate large values. | `-1e30` in HEX mode panics in a debug build (host-run); in release it wraps. `1e30` shows `0x7FFFFFFFFFFFFFFF` with no warning; `0.5` shows `0x0`. | Use `unsigned_abs()`. Show an error or a marker for out-of-range and non-integer values. | Host-run |
| Low | `src/display.rs:56-58` | Scientific format keeps trailing zeros and only 8 significant digits. The `replace` calls are no-ops because Rust never emits `e+` or zero-padded exponents. | `1e10` shows `1.0000000e10`; the exact integer `123456789012` shows `1.2345679e11`. | Trim mantissa zeros; raise the fixed/scientific threshold to about 1e12; delete the dead `replace` chain. | Host-run |
| Low | `src/app.rs:282` | Memory recall in ALG inserts the value with `{}`, which never uses an exponent. | Recalling `1e300` inserts a 301-character literal. Recalling NaN or infinity inserts `NaN`/`inf`, which fail to parse. | Insert via `format_number`, or insert a token such as `M3`. | Host-run |
| Low | `src/app.rs:437-455` | Each ALG-to-RPN toggle pushes ans onto the stack. | Toggling modes twice shifts the stack and drops T. | Push only when the value differs from X, or keep the two modes separate. | Read |
| Low | `src/keymap.rs:296-301` | In the MEM menu the digit pressed is discarded and the app then waits for a second digit. | F4, 3 does not recall M3; it asks for another digit. | Recall the pressed register directly. | Read |
| Low | `src/functions.rs:221-228` | The tan pole check `cos.abs() < 1e-15` fails at large multiples, and the message says "90°" in every angle mode. | `tan(810000000090)` returns about 1,063,648 instead of an error. | Reduce the argument modulo the period before testing. | Host-run |
| Low | `src/functions.rs:468-476` | `0^-1` reports Overflow, not divide-by-zero. | Cosmetic. | Special-case a zero base with a negative exponent. | Host-run |
| Low | `src/main.rs:122-128` | State is saved (PDDB write plus sync) on every background transition, even when nothing changed. | Slow app switching and needless flash wear. | Keep a dirty flag. | Read |

## Improvements

1. Extract `calc-core` (functions, algebraic, rpn, display, memory) as a sub-crate, as writer and timers already do, so `cargo test -p calc-core` runs on host. Add tests for multi-step RPN, unary minus and round-tripping the formatter.
2. No implicit multiplication: `2pi` and `2(3)` are syntax errors. Add it, or show a clearer error.
3. Starting an expression with an operator (`+2`) should prepend `ans`.
4. RPN lacks exponent entry, change sign, roll up and any function access. `RollUp` exists but is never mapped.
5. README claims that do not match the code: "History Tape: view and recall" (display only, ALG only, not persisted); the whole Shift layer; F-key menus; HEX/OCT/BIN are display-only with no entry in those bases.
6. Dead code and dependencies: `KeyState.waiting_store/waiting_recall/fn_menu`, `display::parse_number`, `format_entry`, `app::map_fn_menu_key`; unused crates `libm`, `xous-ipc`, `ticktimer-server`. `#![cfg_attr(target_os = "none", no_std)]` never applies on Xous (target_os is `xous`), and the code uses `std::io` anyway.
7. About 30 TextViews are posted per redraw, each with the default border (visible in the screenshot). Set `draw_border = false` and redraw only the changed region.
8. `%` is modulo with truncated-remainder sign (`-7%3 = -1`). Most calculator users expect percent; document it or rename the key.
9. Persist the RPN stack and history, with a format version byte.

---

# writer

**Verdict:** good architecture (pure core, 40 passing tests, coherent modes), but one emoji keystroke crashes the editor and shortening a document corrupts it on reload. **Health: 4/10.**

## Bugs

| Sev | Location | Description | Failure scenario | Fix | Check |
|---|---|---|---|---|---|
| Critical | `writer-core/src/buffer.rs:50-58`; also `:61-66`, `:78-81`, `:93`, `:102-143` | `cursor.col` is used as a byte index but moves by 1 per character. After any multi-byte character the cursor sits inside a UTF-8 sequence, and `String::insert`, `remove` and slicing panic. | On the stock QWERTY map, Shift+`.` types `😊` and hold-`,` types `福` (`qwerty.rs:54,56`). Type `😊` then any letter: panic "not a char boundary". The process dies and unsaved text is lost. Also reproduced: `é` then Backspace; `福` then Enter; loading `héllo` and pressing Right, Right, then typing; Down onto a shorter line containing accents. Affects Editor and Journal. | Keep `col` as a byte offset but move it by `ch.len_utf8()` and step with `char_indices` (or use `floor_char_boundary`). Clamp to a boundary in `move_up`/`move_down`. Add non-ASCII tests. | Host-run |
| Critical | `src/storage.rs:43-51`, `src/storage.rs:143-149` | Saves write at offset 0 and never truncate; PDDB keeps the old tail. | Save "long content here", delete text so it reads "short", save, reopen: content is **"shortcontent here"**. Every edit that shrinks a document or journal entry brings deleted text back. | `delete_key` then write, or store an explicit length header (`[u8 version][u32 len]...`) and honour it on load. | Host-run (serialisation); Read (PDDB) |
| High | `src/journal.rs:41-46` with `src/storage.rs:143-149` | A journal entry can never be cleared. Writing zero bytes changes nothing, and `load_journal_entry` returns the old text. | Delete all text from today's entry, navigate away, come back: the old entry is still there. | Same fix; delete the key (and its index line) when the entry is empty. | Read |
| High | `src/storage.rs:128-139` | `read_to_string` fails on invalid UTF-8 and the entry is reported as absent. A stale tail can begin mid-character. | Shorten a journal entry that contains non-ASCII text. On the next load the entry appears blank, and typing then saving overwrites the start of the real entry. | Read bytes and use `from_utf8_lossy`. Tell "missing" apart from "unreadable" and refuse to overwrite the latter silently. | Read |
| High | `src/export.rs:47-56`, `src/main.rs:1040` | TCP export calls `accept()` on the main thread with no timeout or cancel, bound to `0.0.0.0:7879` with no authentication. | Choose TCP export with no client ready: the app stops handling keys, redraw and focus messages until something connects. Any host on the network that connects first receives the plaintext document. | Run export in a worker thread with a timeout and a cancel key. Show the device IP and a one-time code, or restrict to a peer the user confirms. | Read |
| High | `src/render.rs:252-360`, `:542`, `:742` | No line wrapping and no horizontal scroll. Each buffer line is one single-height TextView. | In Typewriter mode ("type freely") everything after roughly 40 characters on a line is off-screen. The same applies to Editor and Journal paragraphs. | Soft-wrap in `writer-core` (map buffer lines to visual rows, make the viewport count visual rows) or let the TextView grow and measure it. | Read |
| High | `src/main.rs:860-870`, `src/main.rs:962-967` | `d` in the document list, and "Delete Current" in the file menu, delete immediately with no confirmation. `d` sits next to ordinary navigation keys. | One stray `d` in the list permanently deletes the highlighted document. | Add a confirm dialog like the existing ConfirmExit. | Read |
| Medium | `src/main.rs:992-1002` | Rename saves under the new name without checking whether it exists, then deletes the old key. | Renaming "Notes" to the name of another document overwrites that document (and inherits its tail). | Reject or confirm when the name is already in the index. | Read |
| Medium | `src/render.rs:364-368` | Cursor x is `col * 8`, but the Regular font is proportional and `col` is a byte count. | In `screenshots/typewriter.png` the caret sits well to the right of "Planni". The gap grows along the line. | Monospace for editing, or measure the prefix width through GAM text bounds. | Read (screenshot) |
| Medium | `src/main.rs:262-266`, `src/main.rs:661-795` | All Esc-prefix commands depend on `U+001B`. The Precursor keymaps have no Esc key (zero matches in `qwerty.rs`); it only arrives by serial or hosted mode. | On hardware the "settings in any mode" commands (Esc+A, L, 0, 1, 2) cannot be reached at all; they are not in the F1 menu. The README presents Esc as the main command key. | Add settings to the F1 menu and choose an on-device prefix key. | Read |
| Medium | `src/journal.rs:110-114` | Time defaults to 0 when the RTC is unset. | Before time is set, the journal opens on **1970-01-01** and files the entry there. | Detect `None`, tell the user, and do not create the entry. | Read |
| Medium | `src/ui.rs:4-11`, `src/render.rs:696` | `truncate_str` slices by byte index. | A journal search hit whose line has a multi-byte character across byte 25 panics while drawing results. | Truncate with `char_indices().nth(n)`. | Read |
| Medium | `src/main.rs:823`, `src/main.rs:1186-1189`, `src/main.rs:540-543` | The Typewriter buffer is never persisted and is discarded by `q` or F4 on the summary screen without confirmation. Autosave does not cover it. | A long freewrite is lost to one keypress, a crash or a reboot. | Autosave to a `_typewriter_draft` key on background and confirm before discarding. | Read |
| Medium | `src/main.rs:1166-1169`, `src/main.rs:1012-1015` | The `!ch.is_control()` catch-all accepts arrow characters. | Arrow keys in Typewriter mode or the Rename dialog insert literal `←↑→↓` into the text or name. | Filter U+2190..U+2193 and the PUA range. | Read |
| Medium | `src/main.rs:929-932` | Preview mode ignores every key, so it cannot scroll. | Preview shows only the 13 lines around the edit cursor. | Handle Up/Down/PageUp/PageDown by moving `viewport_top`. | Read |
| Low | `src/main.rs:1266-1271`, `src/journal.rs:42` | Autosave on background and journal day navigation write and sync even when nothing changed. | Slow app switching and day browsing; needless flash wear. | Save only when `modified`. | Read |
| Low | `src/storage.rs:50`, `:120`, `:148`, `:190` | Write errors are discarded and `modified` is cleared regardless. | On a full disk or lost basis the UI shows the document as saved. | Propagate a `Result`; keep `modified` set and show the failure. | Read |
| Low | `writer-core/src/buffer.rs:37` | `str::lines()` drops a trailing empty line. | `"a\n\n"` reloads as `"a\n"`: one trailing blank line is lost per save/load cycle. | Use `split('\n')`. | Host-run |
| Low | `writer-core/src/serialize.rs:171`, `:177` | Day is accepted up to 31 for every month; years before 1970 are treated as 1970. | `2026-02-31` is accepted and maps to 2026-03-03. | Validate against the month table and reject years below 1970. | Host-run |
| Low | `src/main.rs:544-546`, `src/main.rs:830-832` | Help text and footer say "F4 Quit / q Quit"; both handlers are empty. | The keys do nothing. | Remove the hint (apps on Xous should not exit anyway). | Read |
| Low | `src/main.rs:1143` | `q` on an empty search query leaves search. | You cannot search for a word starting with `q`. | Use F4 only. | Read |
| Low | `src/render.rs:37`, `:47-48`, `:52` | GAM calls use `expect`/`unwrap`. | Suspected: a redraw racing a focus change, or a very long line exceeding the TextView limit, panics the app. | Use `.ok()` and log. | Suspected |

## Improvements

1. No undo/redo. For an editor with single-key destructive actions this is the biggest missing feature; a bounded operation log in `writer-core` would be host-testable.
2. Export and save give no on-screen feedback (results go to the log only). Add a status-line message.
3. Journal search opens every entry synchronously, returns one hit per date and caps at 10. Fine for dozens of entries; it will stall with hundreds.
4. `viewport_lines` is fixed at 13 regardless of canvas height or heading sizes. Derive it from `screensize`.
5. Opening a document that fails to load shows an empty buffer under the same name, with autosave on. Mark it read-only or show an error instead.
6. Add a format version byte to documents and config. Document key names carry user text (`doc_{name}`) with no length or character validation against PDDB key limits.
7. Housekeeping: `modals` and `xous-ipc` are unused dependencies; `AppMode::JournalNav`, `export_usb_autotype_chunked`, `set_autotype_delay` and `ExportSystem.tt` are dead. The chunked export splits on byte boundaries and would corrupt multi-byte characters if it were ever used. `char_count` counts bytes.
8. USB autotype of text the host layout cannot type is unchecked.
9. As in calc, every TextView has the default border, which boxes each text line (visible in `screenshots/typewriter.png`).

---

# timers

**Verdict:** the most solid of the three (timestamp-based core, correct F-keys, pump stops when idle), but a timer does not alert when its screen is not in front, which undercuts the app's purpose. **Health: 6/10.**

## Bugs

| Sev | Location | Description | Failure scenario | Fix | Check |
|---|---|---|---|---|---|
| High | `src/main.rs:1169-1172`, `src/main.rs:211-248` | Expiry is detected only inside `handle_pump`, the pump is stopped on background, and only the currently displayed mode is checked. | Start a 25-minute Pomodoro and switch to another app. At 25:00 nothing happens. The alert fires only when you return to Timers. | On background, stop the display pump but schedule a single wake-up at the earliest expiry across all timers. Check every timer in `handle_pump`, not only the visible one. | Read |
| High | `src/main.rs:146-148` | Time comes from `ticktimer.elapsed_ms()`, which upstream documents as not advancing during suspend. | A countdown or Pomodoro that spans a suspend runs long by the suspend duration; the stopwatch under-reports. | Take deltas from the RTC (`llio::LocalTime`) or re-anchor on resume through susres. | Read |
| Medium | `src/main.rs:748-752`, `src/main.rs:406-409`, `src/main.rs:244-246`, `src/main.rs:943-946` | Opening Settings from a running Pomodoro leaves the timer Running; the next pump tick sees Settings mode and stops the pump. Settings then exits to ModeSelect, not back to Pomodoro. | Run a Pomodoro, press `s`, `q`, re-enter Pomodoro: the display is frozen and expiry never fires. The first Enter pauses; the second resumes. | Derive pump state from `any_timer_running()`, not from the mode. Return to the previous mode from Settings. | Read |
| Medium | `src/main.rs:898-902`, `src/main.rs:876-887`, `src/main.rs:574-577`, `src/main.rs:540-547` | Countdown reset creates a `Stopped` timer, but Enter/F2 only handle `Running` and `Paused`. | Press `r` during a countdown: it shows the full duration and Enter does nothing. The only way out is `q` and starting again. | Treat `Stopped` like `Paused` in the start/pause handlers. | Read |
| Medium | `timer-core/src/lib.rs:82-91`, `src/ui.rs:290` | `lap()` zeroes the accumulator, and the main display shows `elapsed_ms`. | Lap at 3.0 s; one second later the main readout shows 1.0 s. Total elapsed time is never shown again. | Keep the total running and store lap boundaries; show total plus current split. | Host-run |
| Medium | `src/main.rs:954-1012` | Pomodoro settings are not validated. Zero is accepted, a parse failure silently becomes the default, and `mins * 60 * 1000` can overflow. | Set all three durations to 0 and start: each tick expires at once and raises a blocking notification, in an endless chain until reboot. The zero settings are saved. | Clamp to 1..=180 minutes and cycles to 1..=12; keep the current value on parse failure; use `checked_mul`. | Host-run (zero duration expires at once); Read (rest) |
| Medium | `src/alerts.rs:25`, `src/main.rs:1083-1086` | `show_notification` blocks the main loop until dismissed, while the pump keeps sending non-blocking `Pump` scalars. `apps/ball` uses a blocking scalar acknowledged with `return_scalar` to prevent this; the README says timers does the same, but the code does not. | Suspected: leave a Pomodoro alert up for a few minutes, then dismiss it. The queued ticks are processed as a burst of full redraws, or sends fail once the queue is full. | Use the ball pattern, or stop the pump before raising the modal. | Suspected |
| Medium | `src/main.rs:1027-1028` | `String::truncate(20)` on user input. | A timer name with a multi-byte character across byte 20 (for example seven `福`) panics. | Truncate on a char boundary. | Read |
| Low | `src/main.rs:621-624`, `src/main.rs:1162-1164` | F4 at the top level terminates the process. Help also lists `q Quit`, which is not handled. | Suspected: after quitting, the Timers menu entry is dead until reboot, since Xous does not relaunch app processes. | Remove quit; return to the launcher or do nothing. | Suspected |
| Low | `src/main.rs:1046-1049`, `src/countdown.rs:28-31`, `src/main.rs:1055-1072` | Invalid duration (`1:2:3`, non-numeric, zero) or a 21st timer is dropped silently after the user typed both fields. Seconds of 60 or more are accepted. | The new timer does not appear and no reason is shown. | Show a notification and re-prompt; check the limit before prompting. | Read |
| Low | `src/main.rs:1082-1086` | The pump sleeps a fixed interval and then sends, so the tick period is interval plus latency. | The MM:SS readout occasionally skips a second; expiry is detected up to a second late. Elapsed time itself does not drift. | Sleep until the next whole-second boundary of the running timer. | Read |
| Low | `src/main.rs:421`, `src/ui.rs:276+` | The stopwatch does a full-screen redraw (clear plus all TextViews) at 10 Hz while showing centiseconds. | High CPU and battery use for digits that cannot be read at that rate. | Redraw only the time TextView; drop to tenths at 4-5 Hz. | Read |
| Low | `src/main.rs:753-760` vs `src/main.rs:602-606` | `q` in Pomodoro or Stopwatch pauses silently and leaves; F4 asks for confirmation. | Inconsistent. | Share one code path. | Read |
| Low | `src/ui.rs` (44 `expect` calls) | Every GAM call panics on error. | Suspected: a redraw racing a focus change kills the app. | Use `.ok()` and log. | Suspected |
| Low | `src/storage.rs:18` | `try_mount()` in the constructor at process start. | Suspected: same boot-time issue as calc. | Load lazily after mount. | Suspected |

Not affected by the stale-tail bug: the Pomodoro and alert records are fixed-size, and the countdown list is count-prefixed, so leftover bytes are ignored.

## Improvements

1. Let timers run concurrently (a Pomodoro alongside a stopwatch) and across mode switches. `TimerCore` already supports it; only the pump and mode coupling prevent it.
2. Persist running timers as wall-clock deadlines so a reboot does not lose a Pomodoro; persist `total_completed` per day.
3. `timer-core` tests do not cover `lap`, zero-duration timers or pause/resume accumulation across several segments. `PomodoroState`, `parse_mmss` and the countdown serialisation are pure logic and belong in `timer-core` with tests.
4. The Audio setting is a toggle that does nothing (README admits this). Hide it until implemented.
5. `TimerState::Expired` is never set. Use it or remove it.
6. `xous-ipc` is an unused dependency; the `#![cfg_attr(target_os = "none", ...)]` lines are dead.
7. The time is centred by padding with spaces (`"     {}"`). Compute x from the canvas width.

---

# Cross-cutting

1. **PDDB writes never truncate.** Calc settings (JSON) and writer documents and journal entries are corrupted whenever the new value is shorter than the old. `xous-dev-toolkit/agents/storage.md:89` recommends `key.set_len(pos)`, which does not exist in upstream PDDB, so the toolkit pattern does not compile and apps that skip it corrupt data. Recommend one shared helper (`pddb_put(dict, key, bytes)` doing delete, write, sync, with a version byte) and a toolkit update. Other slices likely have the same bug.
2. **`try_mount()` in constructors at process start** in all three (`calc/src/storage.rs:60`, `writer/src/storage.rs:22`, `timers/src/storage.rs:18`). Standardise on lazy load after `is_mounted_nonblocking()` or a wait thread as vault does.
3. **Byte indices treated as character indices** (writer buffer, `truncate_str`, timers `name.truncate(20)`). The stock keymap emits `😊` and `福`, so this is reachable without any special layout. Worth a workspace-wide grep for `.truncate(`, `[..n]`, `.insert(` and `.remove(` on user text.
4. **Key-code inconsistency.** Calc uses hosted-mode PUA codes for F-keys and a non-existent shift code; writer and timers use the correct `U+0011..0014`. Writer depends on Esc, which the device does not have. A shared `precursor-keys` constants module (F1-F4, arrows, backspace 0x08/0x7F) would prevent this.
5. **Apps that exit.** Calc exits on any unmapped key; timers exits on F4; writer advertises quit but does nothing. Pick one policy; "never terminate" fits Xous best.
6. **`UxType::Chat` with raw keys** in all three. The canvas loses the input-field area (the "Ready for input..." strip in the screenshots). Calc ignores the real canvas size; writer and timers honour it. `UxType::Framebuffer` gives the full area.
7. **Default TextView borders** on every text line in calc and writer screenshots. Set `draw_border = false` in a shared `post_text` helper; writer's `Renderer::post_text` is a good base for a shared crate.
8. **Unconditional save and sync on focus loss** in calc and writer. Gate on a dirty flag.
9. **Tests pass but do not probe edge cases.** The pure-core split is the right design; calc should adopt it. Add non-ASCII, multi-step and boundary cases. None of the READMEs' test commands work from a standalone checkout: `-p writer-core` needs the xous-core workspace, and calc documents no command.
10. **Boilerplate:** the dead `cfg_attr(target_os = "none", ...)` lines, unused dependencies (`xous-ipc` in all three; `libm` and `ticktimer-server` in calc; `modals` in writer), and `destroy_server` needing `unsafe`.
