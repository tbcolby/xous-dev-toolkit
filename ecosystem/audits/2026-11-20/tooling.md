# Slice: tooling (xous-dev-toolkit, precursor-apps, PrecursorLoRa)

Audited against `SCRATCH/xous-core` (upstream HEAD 31fbe23, xous 0.9.71). All paths below are relative to `SCRATCH/precursor/` unless they start with `xous-core/`.

Method: read every Python and shell script in full; `python3 -I -m py_compile` on all five toolkit scripts (all compile; pyflakes is not installed, so unused imports were found with a small `ast` pass); checked every Rust snippet in the agent/CLAUDE docs against the real signatures in `xous-core/services/{gam,pddb,modals,com,net,trng,keyboard}`, `xous-core/libs/{ux-api,blitstr2,tls}` and `xous-core/emulation/peripherals/keyboard.cs`; parsed `precursor-lora.kicad_pcb` with a throwaway s-expression script (`SCRATCH/tooling_work/pcbcheck.py`) to cross-check nets, pads and track geometry. Nothing was executed against Renode or hardware, and no source repo was modified.

Scores: **xous-dev-toolkit 6/10**, **precursor-apps 4/10**, **PrecursorLoRa 2/10**.

---

## Headline answer: does the toolkit help produce apps that build?

Partly, and the lint data shows exactly where the line is.

| Group | Apps | Errors vs current upstream | What they import |
|---|---|---|---|
| Built with the toolkit's documented pattern (`gam::menu::*`), and have a `capture_*()` function in `renode_capture.py` | c64, calc, carse, flashcards, othello, timers, writer (+ hwtools) | 1 to 4 each, almost all the single `xous::destroy_server` unsafe drift | `gam::menu::*` / `ux_api::minigfx` |
| "32-app campaign" wave apps | barcode, budget, contacts, life, mines, keygen, otp, zork, ... (about 30) | 27 to 93 each | `graphics_server::...` with no such dependency, `gam::RawkeysEvent`, `gam.clear()`, `gam.draw_textview()`, `Canvas`, `Ticktimer::set_alarm`, `xous::String::from_str` |

Findings that follow from this:

1. **The toolkit's own guidance is still correct on current upstream.** `gam::menu` still re-exports `ux_api::minigfx::*` and `Gid` (`xous-core/services/gam/src/menu.rs:6-7`), and `gam` re-exports `GlyphStyle` and `PixelColor`. None of `graphics_server`, `RawkeysEvent`, `draw_textview`, `Canvas`, `set_alarm` appears anywhere in the toolkit or precursor-apps docs (grepped). The wave apps did not follow the toolkit; they were written against an API that is in neither the docs nor upstream HEAD.
2. **The pipeline's quality gates were not enforced.** `ecosystem/PIPELINE.md:83-86` requires a clean `cargo build -p {name}` with zero warnings, `ecosystem/STANDARDS.md` section 13 requires screenshots and an updated `renode_capture.py`, and `ecosystem/TRACKER.md` marks all 32 apps "8/8 SHIPPED". But `renode_capture.py` only dispatches 7 apps (`scripts/renode_capture.py:749-761`), and sampled wave repos (precursor-life, -mines, -keygen, -zork, -budget) contain zero screenshots while precursor-calc has 19 and precursor-timers 4. Suspected (strongly): stages 4 and 5 were never run for the wave apps, on any xous-core version. This cannot be proven from a shallow clone of HEAD, but the missing-crate errors (`graphics_server` used without a Cargo dependency) are independent of upstream version.
3. **Where the docs themselves seed errors seen in lint:** `xous::destroy_server(sid).unwrap()` without `unsafe` (templates in precursor-apps), and `key.set_len()` (`agents/storage.md:89`, 11 lint errors). Details below.

So: the toolkit works when it is actually in the loop. What is missing is anything that mechanically forces it into the loop (see "Missing tooling").

---

## 1. xous-dev-toolkit

**Verdict:** useful Renode automation and a mostly accurate knowledge base, undermined by wall-clock-sleep timing with no verification, scripts that always exit 0, doc snippets that do not compile, and three stale copies of the keyboard guidance that contradict each other. **6/10.**

### 1.1 Python bugs

| Sev | file:line | Description | Failure scenario | Fix |
|---|---|---|---|---|
| High | `scripts/renode_lib.py:438-452` | `start_renode()` probes the monitor port with a bare connect and never checks `proc.poll()`. | A stale Renode from an earlier aborted run still holds port 4567. The new process fails to bind and exits, the probe succeeds against the old instance, and the whole run drives (and at the end `quit`s) the wrong emulator with an old image. | Before `Popen`, fail if the port is already open; inside the loop, abort if `proc.poll() is not None`. |
| High | `scripts/renode_capture.py:717-723`, `scripts/capture_calc.py:88-94` | `ctl.connect()` and `ctl._send('start')` run before the `try:` whose `finally` terminates Renode. | `connect()` raises `RuntimeError` after 3 retries; Renode is orphaned holding the port, which then triggers the bug above on the next run. | Move `start_renode` result into a `try/finally` (or make `RenodeController` a context manager that owns the process). |
| High | `scripts/renode_lib.py:194-249`, `scripts/renode_capture.py:703-772` | `screenshot()` returns 0 on failure and every caller discards it; `main()` has no failure path. | Wrong PIN, wrong app index or a crashed app yields a run that prints "Done!" and exits 0 with missing or wrong PNGs. Unusable as a CI gate. | Count failures, exit non-zero; optionally assert a per-step hash differs from the previous step. |
| High | `scripts/renode_capture.py:177-180` (`esc_cmd`), used at 231-263 | Sends `timed_key('Escape')`. `KeyScanCode.Escape` is not in the keyboard peripheral's `MatrixMapping` (`xous-core/emulation/peripherals/keyboard.cs:278-339`, unmapped keys are dropped at 182-186). | Escape is a no-op, so `esc_cmd('P')` types a literal `p` into the document. Writer's preview, export-menu and journal-exit captures cannot be what their filenames claim. See also the design issue in 1.3 (no ESC on the device at all). | Give Writer a reachable command prefix (F-key or the menu key) and drive that. |
| Medium | `scripts/renode_lib.py:196-206`, `251-268`, `281-307` | Each screenshot sleeps 4 s then reads until a 10 s socket timeout, so costs at least 14 s. `wait_for_screen_change` only adds `interval` to `elapsed`. | `timeout=60, interval=5` really takes about 4 minutes (12 polls at ~19 s). The 19-shot calc capture spends about 4.5 minutes just waiting on timeouts. | After the first chunk drop to a short timeout (0.3 to 0.5 s), or read until the image terminator; measure elapsed with `time.monotonic()`. |
| Medium | `scripts/renode_lib.py:281-307` | Change detection hashes the whole PNG. The status bar renders uptime (`xous-core/services/status/src/main.rs:315-319`; the toolkit's own `discoveries.md` describes uptime visible in screenshots). | Suspected: returns True on the first poll regardless of app state, so it cannot detect "format finished" or "dialog dismissed". Probably why `init_pddb` does not use it. | Crop the status bar before hashing (needs a PNG decode), or compare only the content region. |
| Medium | `scripts/renode_lib.py:365-367` | `init_pddb` sleeps a fixed 15 x 60 s although docs say format takes about 6 minutes, and never confirms completion. | Every `--init` wastes about 9 minutes; on a slower host it can still be too short and the PIN is injected into the progress dialog. | Poll with a working `wait_for_screen_change` and a hard cap. |
| Medium | `scripts/renode_lib.py:112`, `198-211` | Monitor output is never drained except inside `screenshot()`, and the regex takes the first `inline=1:` match in everything buffered. | A screenshot whose data arrives after the read window is left in the buffer and is returned as the result of the next screenshot (off-by-one frames, silently). | Drain before sending `TakeScreenshot`; take the last match. |
| Medium | `scripts/renode_lib.py:157`, `168`; `scripts/renode_capture.py:300` | Text is interpolated into `InjectLine "{text}"` unescaped. | A PIN or sample text containing `"` or `\` breaks the monitor command or injects a different string. | Escape `\` and `"`, or inject per character with `InjectKey`. |
| Medium | `scripts/usb_log_monitor.py:76-81`, `257` | Auto-detect prefers `/dev/tty.usbmodem*` over `/dev/cu.usbmodem*`. | On macOS, opening the `tty.*` node can block waiting for carrier; the tool appears to hang. `cu.*` is the correct node for outbound connections. | Put `cu.usbmodem*` first on macOS. |
| Low | `scripts/renode_lib.py:135` | `f'0:0:0.{hold_ms:03d}'` | `hold_ms=1000` produces `0:0:0.1000` (0.1 s, not 1 s). Docstring says "must be < 500" but nothing enforces it. | Validate range or format seconds properly. |
| Low | `scripts/renode_lib.py:131-140` | No `try/finally` around pause/press/release/start. | A socket error mid-sequence leaves the emulation paused with the key held. | Wrap, always send Release + `start`. |
| Low | `scripts/renode_lib.py:38`, `scripts/renode_capture.py:740`, `scripts/capture_calc.py:79` | Hardcoded `/Volumes/PlexLaCie/Dev/Precursor/...` defaults. `renode_capture.py:740` ignores `XOUS_ROOT`/`--xous-root` entirely; `capture_calc.py:79` uses the module constant, not `--xous-root`. | With the external drive unmounted (the current state), `os.makedirs` fails with a confusing `/Volumes` permission error, or screenshots land in an unexpected tree. No "XOUS_ROOT does not exist" check anywhere. | Require `--screenshots` or default relative to cwd; validate `xous_root` up front. |
| Low | `scripts/renode_lib.py:435`, `455` | Library function calls `sys.exit(1)`. | Importers cannot recover or clean up. | Raise. |
| Low | `scripts/renode_lib.py:440` | `path add @{xous_root}` unquoted; stdout/stderr sent to DEVNULL. | Paths with spaces break; Renode startup errors (bad .resc, missing image) are invisible and surface only as "monitor not ready after 30s". | Quote; log to a file. |
| Low | `scripts/renode_lib.py:386-416` | Apps are launched by menu index. | The submenu order changes with whichever apps are in the image, so every capture needs a hand-found `--app-index`; a wrong index silently captures a different app. | Derive the index from `apps/manifest.json` plus the build's app list, or launch via shellchat/`switch_to_app`. |
| Low | `scripts/renode_lib.py:251` | `tmp_path` parameter is unused. | Dead parameter. | Remove. |
| Low | `scripts/renode_interact.py:61-62`, `113-116`, `148`, `206`, `294`, `34-39` | Deprecated script still shipped and still the first thing README tells users to run. `settimeout` is set after `connect`; no PNG validation; `press-key` with no argument raises `IndexError`; uses scancode `Return`, which is not in the peripheral map (the map has `Enter`), so "dismiss notification" in `init_pddb` cannot work; the `DeprecationWarning` with `stacklevel=2` at module scope is unlikely to be shown. | README quick-start (`README.md:39-48`, `126`) leads to a PDDB init that stalls. | Delete the script or reduce it to a thin CLI over `renode_lib`. |
| Low | `scripts/usb_log_monitor.py:29`, `31`, `32`, `192`, `169-183`, `160` | Unused `os`, `select`; `import serial` at top means `--help`/`--list` crash without pyserial; bare `except:`; `--filter` also filters what `--save` writes; UTF-8 decoded per chunk so a multibyte character split across reads becomes U+FFFD. | Minor. | Lazy-import pyserial with a clear message; save unfiltered; use an incremental decoder. |
| Low | `scripts/capture_calc.py:19` | Unused `DEFAULT_PIN`; whole script duplicates `capture_calc()` in `renode_capture.py:284-443`, which in turn re-implements `inject_key`/`inject_string` (293-301) that already exist on the controller. | Drift between copies. | Delete one. |

No `requirements.txt`/`pyproject.toml`, no tests, no CI in the repo.

Worth knowing: the keyboard service's inject path decodes ANSI escape sequences (`xous-core/services/keyboard/src/main.rs:906-960`, `esc_match` at 1142: `ESC [ A/B/C/D` map to the arrow characters, and a home sequence exists). `precursor-apps/reference/discoveries.md` says arrows cannot be injected. If `InjectKey 27 / 91 / 66` works in practice, arrows and Home could go through injection and `timed_key`'s pause/RunFor dance would only be needed for chords. Untested here; worth a ten-minute experiment.

### 1.2 Doc errors versus real APIs (verified against source)

| Sev | Doc location | Claim | Reality | Consequence |
|---|---|---|---|---|
| High | `agents/graphics.md:251`, `314` | Header bar and selected list rows use `tv.invert = true`. | GAM refuses inverted text unless the canvas trust level is at least `BOOT_CONTEXT_TRUSTLEVEL - 1`, which is granted only to the rootkey and PDDB password modals (`xous-core/services/gam/src/main.rs:263-268`, `contexts.rs:265-272`). It logs "Attempt to draw inverted text without sufficient trust level" and skips the render. | The prescribed header/selection pattern draws a black bar with no text. `invert` is referenced in othello `ui.rs`, writer `render.rs`, hwtools `gpio_tab.rs`/`app.rs` (flagging for the owning slices; I did not check which assign `true`). |
| High | `agents/storage.md:81-93` | "Update (overwrite)" pattern ends with `key.set_len(pos)?`. | `PddbKey` has no `set_len` or truncate (`xous-core/services/pddb/src/frontend/pddbkey.rs`). Source of 11 lint errors (barcode and others). | No truncate exists, so rewriting a shorter JSON blob leaves trailing bytes. The correct pattern is `delete_key` then recreate, or a length-prefixed format. |
| High | `agents/architecture.md:452-493`, `discoveries.md:97` | ESC-prefix command pattern "from Writer", keyed on `'\u{001b}'`. | No Precursor key produces 0x1B (`xous-core/services/keyboard/src/mappings/*.rs` contain no such mapping), Renode has no Escape scancode, and the inject path consumes 0x1B as the start of an escape sequence. | The pattern is unreachable on hardware and in Renode. Writer's preview/save/export/quit (`apps/writer/src/main.rs:262`) appear unreachable except possibly via USB serial console; needs confirmation on the device. |
| High | `agents/architecture.md:525-547` | "Rate-limited pump (from Ball)". | The loop blocks in `receive_message(pump_sid)` every iteration, so it emits one Pump per control message. The real ball app re-arms itself with a self-sent `PumpOp::Pump` (`xous-core/apps/ball/src/main.rs:31-36`). | Animation built from this snippet advances one frame per focus change. |
| Medium | `agents/architecture.md:231-241` (also `precursor-apps/CLAUDE.md` pump section) | "Background pump thread" loops forever with no stop control. | Nothing stops it on `FocusState::Background`. | Exactly the battery-draining pump the review checklist warns about. Only the later "controllable pump" variant is safe. |
| Medium | `agents/architecture.md:252-276` | `fn spawn_worker(...) -> xous::CID` returns `tx` (an `mpsc::Sender`). | Type error. | Does not compile. |
| Medium | `agents/storage.md:297-305` | `pddb::PddbError::{KeyNotFound, DictNotFound}`. | No such type; PDDB returns `std::io::Error` (match `ErrorKind::NotFound`). | Does not compile. |
| Medium | `agents/storage.md:409`, `416-417` | `delete_key_list(dict, &vec_of_str, None)`; `read_dict(dict, None)` yielding `(name, data)`. | `delete_key_list(&self, &str, Vec<String>, Option<&str>)`; `read_dict(&self, &str, Option<&str>, Option<usize>) -> Result<Vec<PddbKeyRecord>>` (`xous-core/services/pddb/src/lib.rs:676`, `1111`). | Does not compile. |
| Medium | `agents/system.md:38`, `52`, `69`, `124`, 127-134, `34` | COM calls used as plain values (`if com.is_charging()`, `let (x,y,z,t) = com.gyro_read_blocking()`, ...). | All return `Result<..., xous::Error>`; `get_batt_stats_blocking` takes `&mut self`; `get_standby_current` returns `Result<Option<i16>>` (`xous-core/services/com/src/lib.rs:238-405`). | Nearly every snippet in the file fails to compile as written. |
| Medium | `agents/system.md:143`, `156-157`; `agents/networking.md:389` | `net::Net::new(&xns)`, `net.wifi_on_and_run()`, `net.wifi_off_and_stop()`. | The type is `net::NetManager::new()` (no args); the methods are `connection_manager_wifi_on_and_run()` / `connection_manager_wifi_off_and_stop()`; `wifi_state_subscribe` needs `&mut self` (`xous-core/services/net/src/lib.rs:57-228`). | Does not compile. |
| Medium | `agents/graphics.md:374`, `357`, `536` | `if tv.overflow`; `gam.glyph_height_hint(..)` as a value; `ValidatorErr::new("..")`. | `overflow: Option<bool>`; `glyph_height_hint` returns `Result<usize>`; `ValidatorErr` is a `String` alias (`xous-core/services/gam/src/modal/textentry.rs:15`). | Does not compile. |
| Medium | `agents/randomness.md:29` | `self.trng.get_u32().unwrap_or(0)`. | Fails open. | If the TRNG call errors, `range()` returns 0 forever (0 is always under the rejection threshold): every dice roll, shuffle, key or pad derived from the wrapper becomes deterministic with no signal. For keygen/otp-class apps this should panic or propagate. |
| Medium | `agents/encoding.md:89`, `107` | `auto_fit_size(..).max(2).min(8)` then `(display_width - total_px) / 2` in `usize`. | For total modules above 168 (QR version 37 and up with a 4-module quiet zone) the 2 px floor exceeds 336 px and the subtraction underflows. | Panic in debug, garbage offset in release. Clamp or cap the version. |
| Medium | `CLAUDE.md:218`, `README.md:194`, `agents/testing.md:333` | Enter scancode is `Return`. | The peripheral map has `Enter` and `KeypadEnter` only. | `timed_key('Return')` is a no-op. The scripts' own comments already route around it with `inject_line("")`. |
| Low | `CLAUDE.md:126`, `141`, `45` | `connect(retries=3, delay=2.0)`; `start_renode(xous_root=None, extra_args=None)`; empty "Python Automation" section. | Parameter is `backoff`; there is no `extra_args`. `--script` (custom capture module, `renode_capture.py:743-747`) is undocumented. | `connect(delay=..)` raises `TypeError`. |
| Low | `README.md:13-48`, `126`, `170-183`, `210` | README still presents `renode_interact.py` as the tool, recommends the single-write press/release that `renode_lib.py` calls unreliable, and says CLAUDE.md contains "a complete app template". | CLAUDE.md has no template; `renode_lib.py`, `capture_calc.py`, `usb_log_monitor.py`, `agents/`, `ecosystem/` are not listed. | First-time users start on the deprecated path. |
| Low | `agents/build.md:85-90`, `222`, `agents/review.md` P3, STANDARDS section 13 | Depending on `ux-api`/`blitstr2` directly is "WRONG". | Upstream `apps/hello/Cargo.toml` depends on both and builds; precursor-apps' template does the same. | Not harmful (the gam re-export route also works and insulates apps from the next refactor), but the two repos give opposite instructions. |
| Low | `agents/build.md:126-137`, `discoveries.md` | Add the app to both `default-members` and `members`. | Upstream apps are in `members` only (`xous-core/Cargo.toml:2-31` vs 32+). | Putting 40 apps in `default-members` means one broken app breaks a bare `cargo build`/`cargo check` for the whole tree. |
| Low | `agents/storage.md:19-20`, `142-143`, STANDARDS | Max name lengths 111 / 95. | The checks are `len > LEN - 1` (`xous-core/services/pddb/src/lib.rs:545-548`), so usable maxima are 110 / 94. | Off by one at the boundary. |
| Low | `agents/storage.md:483` | `try_mount()` is a "non-blocking mount check". | It attempts a mount. | Misleading; use `PddbMountPoller::is_mounted_nonblocking()`. |
| Low | `agents/README.md`, `CLAUDE.md` roster | Ten agents listed. | `agents/randomness.md` and `agents/encoding.md` exist but are not in either roster, so the supervisor never routes to them. | Orphaned agents. |
| Low | `agents/randomness.md:65` | `weights.iter().sum()` in `u32`. | Overflow panics in debug. | Use `u64` or checked sum. |

Suspected, not verified: `usb_log_monitor.py:22` and `58-61` claim Renode reports F1 to F4 as 0xF001 to 0xF004; the Renode peripheral routes F1 through the same matrix position as hardware, which maps to 0x11. `agents/storage.md:364` says Vault uses separate bases for passwords/FIDO/TOTP; Vault's basis calls look user-driven rather than per-credential-type.

Checked and correct: `gam::menu::*` import guidance; `Point`/radius/`stroke_width` are `isize`; `UxRegistration` fields; `GlyphStyle` variants and pixel heights; `TextBounds` variants; `GamObjectList::push` returning `Result`; the `Pddb::get` seven-argument form; every `modals` call in graphics.md except the validator constructor; `tls::xtls::TlsConnector` with ureq 2.9.4; every xtask verb and flag in build.md; generated-file paths; the 128 MiB `tools/pddb-images/renode.bin` backing file; `InjectKey/InjectString/InjectLine`; the 500 ms hold delay; arrows as U+2190 to U+2193 and menu as U+2234; `usb console` in shellchat; SERVER_NAME vs APP_NAME (GAM rejects names absent from `EXPECTED_APP_CONTEXTS`, `xous-core/services/gam/src/tokens.rs:84-102`).

### 1.3 Upstream drift

- `xous::destroy_server` is now `unsafe` (`xous-core/xous-rs/src/syscall.rs:1783`). Fix pattern as in upstream ball: `unsafe { xous::destroy_server(sid).unwrap() }` with a safety comment. This single line is the only error in calc, c64, timers, writer, othello and one of two in carse/flashcards.
- STANDARDS section 11 pins `xous 0.9.69 / xous-ipc 0.10.9 / xous-api-log 0.1.68 / xous-api-names 0.9.70 / xous-api-ticktimer 0.9.68`; upstream hello is at 0.9.71 / 0.10.11 / 0.1.70 / 0.9.72 / 0.9.71. Nothing in the toolkit records which xous-core commit the pins correspond to. Record a known-good SHA.

### 1.4 Improvements (priority order)

1. Make capture runs fail loudly: non-zero exit on any failed screenshot, port-in-use check, child-process ownership in `finally`.
2. Replace fixed sleeps with content-region change detection; cut the per-screenshot 10 s tail.
3. Fix the three doc defects that produce wrong apps: inverted text, `set_len`, ESC prefix. Add a "this does not exist" list (`graphics_server`, `RawkeysEvent`, `gam.clear`, `draw_textview`, `set_alarm`) since agents demonstrably reach for them.
4. Compile the doc snippets. Put each fenced Rust block that claims to be complete into a `doc-snippets` crate (or run them as doctests) built in CI against the pinned xous-core SHA. That one mechanism would have caught every "does not compile" row above.
5. Collapse to one source of truth: delete `renode_interact.py` and `capture_calc.py`, rewrite README from CLAUDE.md, one key table.
6. Move per-app capture sequences out of `renode_capture.py` into each app repo (`capture.py` with `run(ctl, dir)`; the `--script` hook already exists) so the toolkit stops growing by 80 lines per app.

---

## 2. precursor-apps

**Verdict:** the oldest layer of guidance; its template and scaffold produce an app that neither compiles on current upstream nor registers with GAM, and it contradicts the toolkit on keys, scancodes and toolchain. **4/10.**

### Bugs

| Sev | file:line | Description | Failure scenario | Fix |
|---|---|---|---|---|
| Critical | `scripts/new-app.sh:60`, `80`; `CLAUDE.md:81`, `106`, `555` | Scaffold and template pass `SERVER_NAME` (`"_Display Name_"`) as `UxRegistration.app_name`, while the documented manifest `context_name` is the plain display name. | `register_ux` returns `Ok(None)` because the name is not in `EXPECTED_APP_CONTEXTS`; the `.unwrap()` on line 89 panics at startup. This is the exact "app not appearing in menu" failure the toolkit's `discoveries.md` documents as lesson 2; the scaffold was never updated. | Emit `gam::APP_NAME_<UPPER>` (generated from the manifest key) for `app_name`; keep `_Name_` for xous-names only. |
| High | `scripts/new-app.sh:130`; `CLAUDE.md:157` | `xous::destroy_server(sid).unwrap();` without `unsafe`. | E0133 on current upstream; the template propagates it to every app. | `unsafe { ... }`. |
| High | `CLAUDE.md:79` vs `110-114` | Template imports only `num_traits::FromPrimitive` but calls `.to_u32()`. | Does not compile (`ToPrimitive` not in scope). `new-app.sh:54` has the correct import, so the two copies of the template already disagree. | Import both; generate the doc template from the scaffold. |
| High | `CLAUDE.md:341` | Arrow keys documented as `'\u{F700}'` to `'\u{F703}'`. | Apps written from this doc never see arrows. Real values are U+2190 to U+2193; the keyboard service explicitly drops the F700 range on inject (`xous-core/services/keyboard/src/main.rs`, "ignore Apple PUA characters"). Toolkit README and `agents/ideation.md:231` say "do NOT use" these. | Correct the table. |
| Medium | `scripts/new-app.sh:47`, `154` | `sed -i ''` is BSD-only; `$APP_NAME`/`$DISPLAY_NAME` are substituted unescaped into the sed program. | Fails on Linux/CI; a display name containing `/` or `&` (for example "I/O Tools", "Q&A") corrupts or aborts generation. No validation that the app name is a legal crate name. | Use a heredoc with variable expansion or `perl -pe`, validate `^[a-z][a-z0-9-]*$`. |
| Medium | `scripts/new-app.sh:156-162` | Scaffold stops at "next steps": it does not add the workspace member or the manifest entry. | The two steps most often done wrong (name mismatch) are left manual. | Do both edits in the script (`jq` for the manifest). |
| Medium | `scripts/new-app.sh:9` | Requires `precursor-apps/xous-core/`, which is gitignored and absent. | Script exits unless the user knows to clone or symlink there; no `XOUS_ROOT` override although the toolkit uses that variable. | Honour `XOUS_ROOT`. |
| Medium | `CLAUDE.md:365-368` | "Yes/No confirmation" implemented as `alert_builder(..).field(None, None).build()`. | That is a free-text entry box, not a confirmation. | Use `add_list_item("Yes")`/`("No")` + `get_radiobutton`. |
| Low | `CLAUDE.md:735` | Scancodes `ShiftLeft`, `ShiftRight`, `Backspace`. | Peripheral map uses `ShiftL`, `ShiftR`, `BackSpace` (`keyboard.cs:278-339`). | Align with the toolkit table. |
| Low | `CLAUDE.md:522-530` vs `reference/discoveries.md:39` | CLAUDE.md says nightly / `-Zbuild-std` may be required; discoveries says stable with a custom sysroot. | An agent following CLAUDE.md installs the wrong toolchain. | Keep the discoveries version. |
| Low | `.gitignore` | Ignores `notion_update.md`, a file that lives in the other repo. | Cosmetic; suggests the two repos were once one. | Clean up. |

### Upstream drift

Same as the toolkit: `destroy_server` unsafe, stale version pins (`new-app.sh:34-39`). The scaffold's direct `ux-api`/`blitstr2` dependencies are fine on HEAD (matches upstream hello).

### Improvements

1. Decide whether this repo still has a purpose. It is a single commit, superseded by xous-dev-toolkit in every area except `new-app.sh`. Recommended: move a fixed `new-app.sh` into the toolkit and archive this repo, or reduce its CLAUDE.md to a pointer.
2. If kept: make the scaffold the only template, and have it produce the toolkit's STANDARDS layout (APP_NAME/SERVER_NAME split, key constants, opcode numbering, focus handling with pump stop).
3. `Precursor_Agent_Bootstrap.md` phase 4 ("validate with hello world, build and run") is the right idea; it is the step the 32-app campaign skipped.

---

## 3. PrecursorLoRa

**Verdict:** a schematic and a script-generated layout that is not manufacturable as committed: same-layer track crossings, a trace through a row of module pads, the module's UART RX tied to 3V3, and BOM/placement files that do not match the board. README calls it "production-ready" while `AGENTS.md:272` says "42 DRC violations remaining". There is no firmware or Xous driver in the repo. **2/10.**

All coordinates are from `hardware/precursor-lora.kicad_pcb` as committed; the layout is exactly what `hardware/route_pcb.py` emits (45 segments).

### Bugs

| Sev | file:line | Description | Failure scenario | Fix |
|---|---|---|---|---|
| Critical | `hardware/route_pcb.py:52` | RF trace runs along y=92.395 from x=145.07 to 159.0 on F.Cu. That is the centre line of the RAK3172's top pad row. | The trace passes through pads 13 to 20, shorting RF to PA7/PA6/PA5/PA4, two GND pads (17, 18), PA8 and PA9. No RF output; SPI pins tied together. | Leave the RF pad away from the module body before turning. |
| Critical | `hardware/route_pcb.py:53` vs `100`; `66` vs `81`; `88` vs `120`; `143` vs `151-152` | Four orthogonal same-layer crossings: RF x +3V3 on F.Cu at (159.0, 97.99); UART x RST on B.Cu at (158.0, 95.81); RST x +3V3 on B.Cu at (163.0, 100.53); BUSY x LED on B.Cu at (161.5, 109.0). | Each is a dead short between two nets. | Reroute; this needs a real router or interactive routing, not hand-typed coordinates. |
| Critical | `hardware/route_pcb.py:87` | RST leaves J1.5 (160.63, 94.54) to the right on B.Cu at the same y as through-hole J1.6 (161.9, 94.54), which is +3V3. | RST shorted to 3V3 at the header. | Exit the pin away from its neighbour. |
| Critical | `hardware/route_pcb.py:82-84`, `104-105` | RST via and track at (158.0/157.265, 96.5) overlap GND pad U2.23 (y=96.72, 0.8 mm tall); +3V3 at x=155.0 and y=104.52 overlaps U2.31 (PB4) and U2.29 (PA0). The x=155 +3V3 run also clears C3.2/C4.2 (GND) by about 0.015 mm. | RST shorted to GND (module held in reset); 3V3 driven into two GPIO pads; near-short 3V3 to GND. | As above. |
| Critical | `hardware/precursor-lora.kicad_sch` (netlist as reflected in the PCB: U2 pad 5 `PB7/UART1_RX` is on net `+3V3`) | The module's UART1_RX is tied to 3V3. Only one UART net exists (`Net-(J1-Pin_1)`: J1.1, J1.7, U2.4 = UART1_TX). `route_pcb.py` likewise has a single `NET_UART` and no net 6. | The Precursor can never send an AT command to the module; the design is receive-only. Contradicts `README.md:35-36` and `docs/assembly-notes.md:72-73`. | Fix the schematic: separate RX net to a header pin. |
| High | `fabrication/bom/bom.csv`, `README.md` BOM, `fabrication/pick-and-place/positions.csv` vs PCB | Reference designators disagree. PCB/schematic: U1 = AP2112K, U2 = RAK3172, R1 = 1K, R2 = R3 = 10K. BOM/README/positions: U1 = RAK3172, U2 = AP2112K, R1 = R2 = 10K, R3 = 1K. `positions.csv` coordinates (for example RAK at 25.0, 15.0) are not derived from the board (the module is at 149.5, 99.9 in a 135..185 x 85..115 outline, i.e. 14.5, 14.9 board-relative). | An assembler following the files places the LDO on the LGA footprint and swaps the LED resistor with a pull-up. | Export BOM and CPL from KiCad; never hand-write them. |
| High | PCB footprint R3 | Both pads of R3 are on the same net (`Net-(J1-Pin_5)`, the reset net). | R3 does nothing. Whatever it was meant to be (series resistor, pull-down) is missing. | Fix in schematic. |
| High | `hardware/route_pcb.py:52-54`, `PrecursorLoRa/CLAUDE.md:49` | RF trace is 0.3 mm wide and about 23 mm long with two right-angle corners, on a 2-layer 1.6 mm board. The repo's own rule is under 10 mm at 50 ohm. | A 0.3 mm microstrip over 1.6 mm FR4 is far above 50 ohm (50 ohm needs roughly 3 mm, or a coplanar design on thinner dielectric): significant mismatch loss on top of the shorts. | Move J2 next to pin 12; use a calculated coplanar waveguide with via stitching. |
| High | BOM J1 vs PCB | BOM lists Samtec FTSH-104-01-L-DV (SMD); footprint is `PinHeader_2x04_P1.27mm_Vertical` through-hole. Symbol is a 1x08 connector mapped onto a 2x04 footprint. | Ordered part does not fit; pin numbering is easy to get mirrored against the Precursor side. | Pick one; verify against the actual Precursor connector. |
| Medium | `hardware/DRC.rpt` | The committed report (2026-01-28, 42 violations, including shorts) describes an older routing (tracks at x=170/172 that no longer exist). | Nobody can tell the board's current state from the repo; the current routing still has the shorts listed above. | Regenerate with `kicad-cli pcb drc` in CI and fail on errors. |
| Medium | `hardware/route_pcb.py:174`, `180-193`, `205-209`, `213` | Hardcoded `/Volumes/PlexLaCie/...` path; rewrites the PCB in place with no backup; deletes every existing segment and via including any manual fixes; on a missing insertion marker prints "ERROR!" and exits 0. No clearance or crossing check despite the "KEEPOUT-AWARE" title. | Running it destroys hand routing and cannot report its own failures. | Take the path as an argument, write to a new file, exit non-zero, or retire the script in favour of interactive routing. |
| Medium | J1 pin 6 = `+3V3` | The board's LDO output is exported on the header; the README pin table does not mention it. | Suspected: if the mating Precursor pin is a 3.3 V rail, two regulators are tied together. | Confirm intent; document or remove. |
| Medium | UART2 (U2 pads 1, 2) unconnected | Suspected: on stock RAK3172 firmware UART2 is the default AT/bootloader port, and UART1 needs configuration. | Possibly no AT console and no firmware-update path once assembled. Verify against the RAK datasheet (the repo's CLAUDE.md already lists the symbol pinout as needing human verification). | Bring UART2 and BOOT0/RST to test pads at minimum. |
| Low | `README.md` directory tree | Lists `fabrication/gerbers/` and `datasheets/`; neither exists. `README.md:3` says "production-ready"; `CLAUDE.md:5-7` says "ready for KiCad verification" and "submit order within ~1 week". | Risk of ordering a board from this state. | Correct the status line first. |
| Low | `hardware/ERC.rpt` | 31 warnings: every symbol mismatches its library copy, three dangling wire endpoints. | Symptoms of a hand-generated schematic file. | Update symbols from library, clean wires. |

### Improvements

1. Do not order. Fix the schematic (UART RX, R3, header pinout), then re-place J2 beside the RF pin and route interactively or with an autorouter (Freerouting), then DRC to zero.
2. Add `kicad-cli sch erc` and `kicad-cli pcb drc` to CI; generate BOM, CPL and gerbers from CI artifacts only.
3. Verify the mechanical premise: the README says the board "fits in the battery compartment" and uses an "internal GPIO header"; neither is backed by a drawing or a reference to betrusted-hardware.
4. The Xous side does not exist: there is no driver, no UART pin mapping in the SoC, no service. State that plainly in the README.

---

## Missing tooling (answers to the brief's questions)

| Gap | Evidence | Proposal |
|---|---|---|
| **CI** | No `.github/` in any of the three repos; 40 of 40 portfolio apps fail to build on upstream HEAD; TRACKER says all shipped. | One portfolio workflow: check out xous-core at a pinned SHA, copy each app in, run hosted `cargo check -p <app>` and `cargo test` for the `*-core` crates. Nightly job against upstream `main` to surface drift early (the `destroy_server` change would have been a one-line fix on the day it landed). Add `python -m py_compile` + ruff for the scripts and `kicad-cli` ERC/DRC for LoRa. |
| **Build-all / integrate script** | The audit had to hand-assemble the tree and found packaging anomalies (c64 `../../xous-core/services/...` paths, othello's lib location, `cipher` and `qrcode` package/manifest name mismatches, see `SCRATCH/path_anomalies.txt`). | `integrate.sh`: copy or symlink every `precursor-*` repo into `apps/`, merge a per-repo `manifest-fragment.json` into `apps/manifest.json`, patch workspace `members`, then build each app and print a pass/fail table. This is also the CI entry point. |
| **Scaffold generator** | `new-app.sh` exists but emits a non-registering, non-compiling app and skips manifest/workspace edits. | Fix it, move it into the toolkit, and add a CI job that scaffolds a throwaway app and builds it, so the template can never rot again. |
| **Shared crate** | STANDARDS mandates identical key constants, header/footer layout, opcode numbering, focus handling and JSON settings in every app; the wave apps each re-invent (and mis-invent) the GAM surface. | `libs/precursor-appkit`: key constants, `clear/header/footer/list_item` helpers built on `gam::menu` (without `invert`), a `Settings<T>` PDDB helper using delete-then-write, the fail-closed TRNG wrapper from randomness.md, a controllable pump. An upstream API change then becomes one fix instead of forty. |
| **Version anchor** | Pins in STANDARDS section 11 with no xous-core SHA. | Record the known-good xous-core commit in the toolkit; CI uses it. |
| **Truthful status** | TRACKER "SHIPPED" is hand-edited. | Generate the status column from CI results. |
| **Python packaging and tests** | No requirements file, no tests. | `pyproject.toml` with a pyserial extra; unit-test the screenshot parser against a recorded monitor transcript. |

---

## Cross-cutting patterns

1. **Documentation is never executed.** Roughly twenty snippets across the agent docs do not compile, and the two repos' templates disagree with each other and with upstream. Compiling the snippets in CI fixes the whole class.
2. **Three generations of guidance coexist** (precursor-apps CLAUDE.md, toolkit README, toolkit CLAUDE.md + agents) with contradictory answers for arrow key codes, Enter and Shift scancodes, key-press technique, toolchain, and whether to depend on `ux-api`. An agent's output depends on which file it happened to read.
3. **"Done" is asserted, not measured.** Capture scripts exit 0 on failure, TRACKER is hand-maintained, LoRa's README says production-ready over a board with shorts, and DRC.rpt is stale. Every one of these is fixed by making a machine produce the status.
4. **Generated artifacts written by hand.** `route_pcb.py` coordinates, `positions.csv`, the BOM, and the precursor-apps template are all hand-typed stand-ins for something a tool should export, and all are wrong in ways the tool would not be.
5. **Hardcoded `/Volumes/PlexLaCie/...`** in `renode_lib.py:38`, `renode_capture.py:740` and `route_pcb.py:174`. With the drive unmounted nothing runs, and the errors do not say why.
6. **Fixed wall-clock sleeps as synchronisation** throughout the Renode automation. Emulation speed varies by host, so runs are both slow (about 15 minutes for PDDB init, about 14 s per screenshot) and flaky.
