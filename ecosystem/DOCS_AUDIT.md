# Docs audit: toolkit vs Xous Book and xous-core source

Audited 2026-11-20 against the Xous Book (https://betrusted.io/xous-book/) and xous-core at the pinned commit `31fbe23`
(local checkout `xc-w3`, identical to `xc-w1`). No toolkit file other than this report was edited. No cargo builds were run.

Rule applied: where the Book and source disagree, source wins. Source paths below are relative to xous-core @ 31fbe23.
Book pages are cited as `chNN-...html` under https://betrusted.io/xous-book/. Anything not checked is marked UNVERIFIED.

## Summary: top 5 actions

1. **Localization: manifest entries with only `en`/`en-tts` panic at runtime in other languages.** `locales/README.md` states omitting a
   locale "will lead to a panic"; `locales/build.rs:~100-115` generates `e => panic!("Missing language: {}", e)`. The template manifest
   fragment, `agents/build.md:103-115` and `discoveries.md:117-130` all show only `en`/`en-tts`. Add `fr/ja/zh` (copy `en`, or the 🔇 marker)
   to the template and `bin/lint`, and document `locales/i18n.json` plus `t!()` (see Gaps G1).
2. **Fix `--app-feature` (`agents/build.md:283-296,310-327`).** It is only for "detached app" builds (dabao), not per-app features
   on Precursor (`xtask/src/builder.rs:1173-1176`, `xtask/src/main.rs:226-229,1142`). The documented `renode-image myapp --app-feature x` is a no-op.
3. **Add the app-loader / detached-app dev loop and the `submenu` manifest field**, both absent from the toolkit
   (`apps/app-loader/README.md`, `xtask/src/app_manifest.rs:11-14,59-70`, `services/gam/Cargo.toml:71`).
4. **Document IPC rules the Book stresses and the toolkit omits**: deferred responses, `try_send_message` vs `send_message`,
   memory messages return on Drop, no re-lending of pages, `lend`/`lend_mut`/`send` semantics (Book ch07-02..ch07-06;
   `xous-rs/src/syscall.rs:1530,1574`, `xous-ipc/src/buffer.rs:133-162`). `agents/architecture.md` only sketches lend/lend_mut.
5. **Correct small factual drifts**: `graphics.md:465` (`update_progress` never returns Err for a full queue), `build.md:247`
   (`locales/src/generated/` is the file `generated.rs`), stale `scripts/new-app.sh` reference in the template
   (`templates/app/src/main.rs:4`), and KeypadEnter (Renode) maps to `∴`, not Enter. Also add a suspend/resume (susres) section.

Counts: Contradictions 7, Gaps 11, Refinements 9, Tooling 7, Verified correct 24.

## Contradictions

| Toolkit file:line | Claim | Official source + citation | Verdict |
|---|---|---|---|
| `agents/build.md:283-296` (also 310-327) | `--app-feature my_feature` = "per-app conditional compilation"; example `renode-image myapp --app-feature debug_overlay` | `xtask/src/main.rs:226-229` collects `--app-feature` into `detached_app_features`; `builder.rs:1171-1177` uses them only in the `board-dabao` "detached apps" branch; help text `main.rs:1142` says `[--app-feature [detached app feature name]]`. Precursor `renode-image`/`app-image` never read it. | **Wrong.** Use `--feature <crate>/<feature>` (generic `--feature`, `main.rs:206-211`) UNVERIFIED for per-package syntax. Source is authoritative; Book does not document it. |
| `agents/build.md:247` | Generated locale output at `locales/src/generated/` (directory) | `locales/src/lib.rs` has `pub mod generated;`; the tree has `locales/src/generated.rs`; `xtask generate-locales` (`main.rs:1050,1246`) | **Wrong (minor).** File, not directory. |
| `agents/graphics.md:463-468` | `update_progress` "may overflow queue"; code treats `Err` as queue-full and yields | `services/modals/src/lib.rs:516-532`: uses `try_send_message`, on failure logs, calls `yield_slice()` itself, then always returns `Ok(())` | **Wrong.** The `is_err()` branch is dead; just call it. |
| `agents/build.md:99-115`, `discoveries.md:117-130`, `templates/app/manifest-fragment.json` | Manifest entry with only `en` and `en-tts` is sufficient | `locales/README.md` ("Omitting a locale entirely ... will lead to a panic"); `locales/build.rs` `panic!("Missing language")`; `apps/README.md` manifest section ("We suggest creating strings for every language ... prevent builds from breaking") | **Incomplete/unsafe.** Upstream `calc`, `writer`, `timers` also have only `en`/`en-tts` in `apps/manifest.json`, so default English builds work, but `--feature xous/lang-fr|ja|zh` builds panic when the menu string is looked up. Source README wins. |
| `CLAUDE.md:70`, `agents/build.md:135-137`, `STANDARDS.md:337` | Add apps to `members` only, never `default-members` | `apps/README.md` step 2 says add to **both** `default-members` and `members`. Source `Cargo.toml:2-31` shows `default-members` contains no app crates (apps are only in `members`, lines 32+). | **Toolkit is consistent with the actual Cargo.toml; the official README step is stale.** Keep the toolkit rule, but note the divergence from `apps/README.md` explicitly so readers are not confused. |
| `scripts/renode_lib.py:82`, `scripts/README.md:82` | `KeypadEnter` listed among valid scancodes (implied as an Enter variant) | `emulation/peripherals/keyboard.cs:337,339`: `KeypadEnter` and `Home` both map to row5/bit2 = (5,2), which is `'∴'` (`services/keyboard/src/mappings/qwerty.rs:70`). Real Enter is `KeyScanCode.Enter` (row7 bit9 -> `'\r'`, qwerty.rs:51). | **Misleading.** `KeypadEnter` opens the main menu. Note it in the docs or remove it from the allowed set. |
| `templates/app/src/main.rs:4` | "generated by xous-dev-toolkit/scripts/new-app.sh" | Script is `bin/new-app` (`bin/new-app`, README.md:11) | **Wrong (stale).** |

Book-vs-source disagreements found while auditing (source wins; toolkit follows source in all of these, so no toolkit change needed,
but do not "fix" the toolkit toward the Book):
- Book ch08-01 shows `modals.show_notification("This is a test!")` (one argument). Source: `show_notification(&self, notification: &str, qrtext: Option<&str>)` (`services/modals/src/lib.rs:238`). Toolkit `graphics.md:522` matches source.
- Book ch08-01 shows `get_text(prompt, validator, validator_op)`. Source has no `get_text`; text entry is `alert_builder(prompt).field(..).build()` (`modals/src/lib.rs:201`) with `TextValidationFn = fn(&TextEntryPayload) -> Option<ValidatorErr>` (`lib.rs:29`). Toolkit `graphics.md:531-550` matches source.
- Book hello-world chapter / ch07 examples use `#[xous::xous_main] fn xmain()`. Source `apps/hello/src/main.rs:1-2,112` and `apps/ball/src/main.rs:38` use plain `fn main() -> !` with `no_main` on `target_os = "none"`. Toolkit template matches source.
- Book ch09-04 says PDDB std names have "no restrictions"; ch09-03 says avoid `:`; source name limits are `DICT_NAME_LEN = 111`, `KEY_NAME_LEN = 95` slots (`services/pddb/src/api.rs:29,31`), i.e. 110/94 usable bytes. Toolkit uses 110/94 (correct).
- Book ch06-00 has a path typo `riscv32imac-unknown-elf` (should be `-xous-elf`).

## Gaps

Each item: what official material says and what the toolkit lacks.

- **G1. Localization (i18n).** App strings: put `locales/i18n.json` in the app dir, read with `locales::t!("key", locales::LANG)`
  (`apps/hello/src/main.rs:7,102`, `apps/hello/locales/i18n.json`, `apps/hello/Cargo.toml` dep `locales = { path = "../../locales" }`).
  Build globs `**/i18n.json` (`locales/build.rs:~28-36`); regenerate with `cargo xtask generate-locales` (`xtask/src/main.rs:1050`);
  select language with `--feature xous/lang-{en-tts,fr,ja,zh}` (`main.rs:206-216`, `locales/src/locale.rs`). `tools/i18n_helper.py` exists.
  Toolkit: no mention of any of this (only `--feature xous/lang-ja` appears in `build.md:295`). appkit has no helper.
- **G2. `submenu` manifest field.** `AppManifest.submenu: Option<u8>` (`xtask/src/app_manifest.rs:11-14`) generates
  `APP_MENU_<i>_<KEY>` constants and adds them to `EXPECTED_APP_CONTEXTS` (`app_manifest.rs:59-70`); used by `app-loader` and `chat-test`
  (`apps/manifest.json`). Needed by any app that registers extra `UxType::Menu` contexts. Toolkit documents only `context_name` and `menu_name`.
  Also: `context_name` has a 64-char limit and all names must be unique (`apps/README.md` manifest section); toolkit does not state either.
- **G3. App loader / detached apps / unsafe-app-loading.** `apps/app-loader/README.md`: `cargo xtask app-image-xip --app app-loader --feature unsafe-app-loading`,
  `cargo xtask compile-apps --app hello ...`, `python3 tools/app_server.py <apps> -p 8000` (tools/app_server.py exists). GAM gate:
  `services/gam/src/tokens.rs:82-102` (`unsafe-app-loading` allows dynamic names; `services/gam/Cargo.toml:71`). Faster on-device iteration than a full image; note "apps can be loaded only once per boot" (README). `compile-apps` verb itself UNVERIFIED in xtask (not grepped).
- **G4. Deferred responses.** Book ch07-06: store `msg.sender` (scalar) or keep the `MessageEnvelope` (memory) and respond later; memory case needs
  `Vec`/`Option::take()`. Not in toolkit (`agents/architecture.md` covers only immediate `return_scalar`, lines 640-662). Needed for blocking calls whose answer depends on a later event (e.g. key press, timer).
- **G5. `send_message` vs `try_send_message`.** Source `xous-rs/src/syscall.rs:1530` (`try_send_message`, non-blocking on full queue) vs `:1574`
  (`send_message`, blocks). Official use: `modals.update_progress` (`modals/src/lib.rs:517`). Book never mentions `try_send_message` (searched all 18 pages).
  Toolkit uses `send_message(..).ok()` everywhere (`architecture.md:314-429`) and does not explain the blocking/queue-overflow trade-off. Document when a non-blocking scalar to a stuck peer can hang the sender.
- **G6. Memory-message semantics.** Book ch07-00/ch07-04/ch07-05: memory messages are page-sized/aligned; lent memory returns to the caller on `Drop`;
  "you can't re-lend pages"; callee uses `buffer.replace(data)` for `lend_mut` results; `Buffer::send` consumes (`xous-ipc/src/buffer.rs:162`),
  `lend` borrows (`:151`), `lend_mut` (`:133`). Toolkit `architecture.md:470-500` shows lend/lend_mut but not `send`, `replace`, drop-return, or the no-relend rule. rkyv version in toolkit (0.8.8) matches `xous-ipc/Cargo.toml:15`.
- **G7. Suspend/resume (susres).** Source API: `xous-api-susres` (`api/xous-api-susres/src/lib.rs:71-124`: `Susres::new`, `suspend_until_resume(token)`, `set_suspendable`, `initiate_suspend`).
  Toolkit mentions suspend only as a timers "suspend-safe time" note (`REMEDIATION.md:46`). No guidance on hooking suspend, flushing PDDB (`sync()`) before sleep, or time jumps after resume (ticktimer `elapsed_ms` vs wall clock). Note upstream apps rarely hook susres (vault lists the dependency in `apps/vault/Cargo.toml:26` but only `Cargo.toml` matched in grep); UNVERIFIED whether any app does.
- **G8. Ticktimer API.** Real surface: `Ticktimer::{new, elapsed_ms, sleep_ms, ping_wdt, get_version, lock_mutex, unlock_mutex, wait_condition, notify_condition}`
  (`api/xous-api-ticktimer/src/lib.rs:14-158`). Toolkit only shows `sleep_ms` in appkit pump and warns `set_alarm` doesn't exist. `elapsed_ms` (monotonic time for deltas, used by LESSONS "ticktimer deltas") and `wait_condition`-with-timeout (a cleaner pump than a sleeper thread) are undocumented.
- **G9. PDDB std API alternative.** Book ch09-04: ordinary `std::fs` paths of the form `dict:key` work on hardware/Renode (not hosted: "only accurate on real hardware and Renode"). Toolkit documents only the native `Pddb` API (`storage.md:41-83`). Mention std::fs as the lighter option and its limits (no truncate-on-open, rename, copy per Book).
- **G10. Hosted mode limits.** Book ch03-02/ch01-02: no tokio, no `File` (use PDDB), `socket2` unsupported, 32-bit `usize`, most syscalls `NotImplemented`, hosted PDDB not representative. Toolkit README tells readers to iterate with `cargo xtask run` but never lists what hosted mode cannot validate.
- **G11. Menus (`menu_matic`) and other modals widgets.** Book ch08-02: `menu_matic`, `MenuItem` (name 64 bytes, `action_conn`, `action_opcode`, `action_payload` scalar-only, `close_on_select`), `gam.raise_menu`. App-level menus are usable by apps (the F1 menu pattern); toolkit `graphics.md` covers modals but not `menu_matic`. Book notes memory payloads are "not yet implemented". Book also states boot-time-menu registration needs an `EXPECTED_BOOT_CONTEXTS` entry (system menus only; irrelevant for apps).
  Also absent: Book ch01-03 coding style ("code in `main` free of rustc lint", `.unwrap()` for infallible ops), thread limits (32 threads/process, 32 connections/process, 128 servers; ch02-00, ch07-01), and signing (`--lkey/--kkey`, `main.rs:1177` help; toolkit shows them in `build.md:304-308` but gives no dev-key vs release-key guidance, UNVERIFIED).

## Refinements

- **R1. Hello vs template main loop.** `apps/hello` registers `UxType::Chat` with `rawkeys_id: None`, `focuschange_id: None` (`hello/src/main.rs:42-52`); `apps/ball` uses `Framebuffer` + `rawkeys_id` + `focuschange_id` (`ball/src/ball.rs:38-46`). The template follows ball (correct for games). Add a Chat/`gotinput_id` variant to the template (appkit `Opcodes.got_input` exists but no template shows it).
- **R2. Opcode enum idiom.** Upstream uses implicit discriminants starting at 0 (`ball/src/main.rs:14-25`, `hello/src/main.rs:21-27`). `STANDARDS.md:133-156` mandates banded explicit values (Redraw=0..., Quit=255). Harmless, but it is a toolkit convention, not official; label it as such. Also upstream Quit is a blocking scalar that acks and breaks (`ball/src/main.rs:111-120`); the template's `Quit => break` (`templates/app/src/main.rs:86`) skips the ack, so a sender blocking on Quit would hang.
- **R3. Unknown-opcode arm.** Upstream logs `log::error!("couldn't convert opcode: {:?}", msg)` and keeps looping; template does the same with `warn`. Fine.
- **R4. Pump shape.** Official ball pump is a blocking scalar so pumps cannot outrun drawing (`ball/src/main.rs:69,77`; `return_scalar(msg.sender, 1)`). appkit `pump::ack` matches this pattern. Confirmed good; no change.
- **R5. Teardown.** Upstream `ball` does `xns.unregister_server(sid)` then `unsafe destroy_server` (`main.rs:126-130`), matching `appkit::app::shutdown`. OK.
- **R6. Wrap `locales`.** Add `precursor_appkit::i18n` (or template `locales/i18n.json` + `t!` usage) so apps are not English-only (G1).
- **R7. Wrap modals.** `appkit/README.md:42` says modals wrappers are intentionally not provided. Official API is stable and small (`show_notification`, `get_text`-equivalent via `alert_builder`, `start/update/finish_progress`, radio/checkbox). A thin `appkit::dialog` returning `Result` and handling the "modal steals focus; redraw on return" rule would match LESSONS "Open: modal confirm/menu overlay".
- **R8. Ticktimer in appkit.** Expose `elapsed_ms` helper instead of every app calling `Ticktimer::new()` (see G8). Pump currently creates its own `Ticktimer` (`appkit/src/pump.rs:93`).
- **R9. Hello-style TTS.** Upstream hello optionally uses `tts-frontend` (`hello/src/main.rs:9-10,34-36,104-105`; `en-tts` locale). Toolkit sets `en-tts` strings in the manifest only; nothing on accessibility. Low priority.

## Tooling

- **T1. Build flow matches official verbs.** `cargo xtask run|renode-image|app-image|app-image-xip|hosted-ci|renode-image-debug` all exist (`xtask/src/main.rs:267,272,419,494,518,525,1202-1236`); the Book (ch06-00) documents `run`, `app-image`, `renode-image`. `bin/integrate` correctly uses `hosted-ci` (a `hosted_build_only` dry run, `builder.rs:649`) only to generate `apps.rs`/`app_autogen.rs`.
- **T2. `HOSTED_PKGS` in `bin/integrate:37-41` equals xtask `user_pkgs` + base set** (`main.rs:104-135`, gfx base at ~95-100). Drift risk: duplicated list. Add a CI step that diffs it against `xtask/src/main.rs` at the anchor (UNVERIFIED that the base-package set `xous-ticktimer xous-log xous-names xous-susres` is complete; names looked right).
- **T3. No locales check.** Add a `bin/lint`/`integrate` rule that every manifest-fragment `menu_name` entry has `en`, `en-tts`, `fr`, `ja`, `zh` (or 🔇), plus a CI job running `cargo xtask generate-locales` and a hosted check with `--feature xous/lang-ja` (see G1, Contradictions row 4).
- **T4. No image-level CI.** CI never runs `cargo xtask renode-image <app>` or `app-image`; the riscv target is untested (README.md:71 notes hosted needs no sysroot; `cargo xtask install-toolkit` exists, `main.rs:257,1248`, and CI already calls it). Add a nightly job building `renode-image` for the scaffold (upstream CI does a hosted run plus images; Renode itself is not required just to build). Check also `renode-test` (`main.rs:289`) as the official Renode test path (Book ch04/ch06-01: not fetched, UNVERIFIED).
- **T5. `README.md:71` claim "Hosted builds need no sysroot" vs CI comment "xtask needs it to generate the GAM app table"** (`.github/workflows/*.yml`, both "Xous toolchain" steps). The workflows install the toolkit for hosted builds, implying the README statement is not fully true for the `integrate --check` flow. Source path not checked; UNVERIFIED which is right.
- **T6. Detached/app-loader path missing** from `bin/integrate`/docs (G3). `compile-apps` and `tools/app_server.py` offer hardware iteration without a full flash.
- **T7. Hosted run instruction.** README step "cd $XOUS_ROOT && cargo xtask run tally" matches Book ch01-02 / ch03-02. The Book's troubleshooting additions are absent from the toolkit: `cargo xtask install-toolkit --force`, `rm -r target`, `libxkbcommon-dev` on Linux (CI installs the latter already).

## Verified correct

Checked against source at 31fbe23:

1. `∴` (U+2234) is consumed by GAM for app contexts and never delivered: `services/gam/src/contexts.rs:741` (`if keys[0] == '∴'`) and `qwerty.rs:70`. CLAUDE.md:34 line citation is exact.
2. Arrow keys U+2190-2193: `qwerty.rs:65-68`.
3. F1-F4 = 0x11-0x14: `qwerty.rs:60-64`.
4. Enter = 0x0D, Backspace = 0x08: `qwerty.rs:40,51`.
5. Shift+`.` gives `😊`: `qwerty.rs:56`.
6. No Escape key and no 0x1B in the map: `qwerty.rs` (no entry). Renode keyboard map has no Escape (`keyboard.cs`).
7. GAM app name must be a generated `APP_NAME_<KEY>` (uppercased, `-` -> `_`) whose value is `context_name`: `xtask/src/app_manifest.rs:59-96`.
8. GAM rejects unregistered names (`claim_token` returns `None`; `register_ux` then yields `Ok(None)`): `services/gam/src/tokens.rs:82-102`. Citation `tokens.rs:84-102` in `build.md:128` is accurate.
9. Generated files: `services/gam/src/apps.rs`, `services/status/src/app_autogen.rs` (`app_manifest.rs:96,185`; also `services/cram-console/src/app_autogen.rs:186`, not listed by the toolkit).
10. Only apps named on the xtask command line get constants (working_set filter, `app_manifest.rs:52-57`).
11. Crate versions: xous 0.9.71, xous-ipc 0.10.11, xous-api-log 0.1.70, xous-api-names 0.9.72, ticktimer 0.9.71: `apps/hello/Cargo.toml`, `apps/ball/Cargo.toml:13-17`.
12. `destroy_server` is `unsafe`; ball wraps it with a safety comment (`ball/src/main.rs:127-130`).
13. `gam.relinquish_focus()` exists and is the leave mechanism: `services/gam/src/lib.rs:409`. (Only `app-loader` calls it among upstream apps, `apps/app-loader/src/api.rs:354`; the "don't terminate" rationale is the toolkit's own and UNVERIFIED against kernel behavior.)
14. `UxType::{Chat, Menu, Modal, Framebuffer}` and `UxRegistration` fields: `services/gam/src/api.rs:80-95`; ball uses Framebuffer + rawkeys (`ball.rs:38-46`).
15. `gam::FocusState::convert_focus_change` and the Background/Foreground handling pattern: `ball/src/main.rs:88-110`.
16. `gam::GlyphStyle` and `PixelColor` re-exports (the "gam re-exports route"): `services/gam/src/lib.rs:18,21,24`.
17. PDDB limits 110 (dict) / 94 (key): `services/pddb/src/api.rs:29,31` (slots 111/95) and Book ch09-03.
18. No `set_len`/truncate on `PddbKey`; only `delete_key` exists (`pddb/src/lib.rs:627`); basis-aware API (`attributes().basis`, `monitor_basis` `lib.rs:302`, `latest_basis` `:323`, `sync` `:784`, `is_mounted_blocking` `:170`). Book ch09-04 confirms truncation of open files is unsupported.
19. `None` basis semantics (reads union, new key to most recently opened basis): Book ch09-03.
20. Modals widgets documented in `graphics.md:451-552` exist: `slider` (`modals/src/lib.rs:452`), `add_stateful_list_item` (`:626`), `add_list_item` (`:618`), `get_checkbox` (`:749`), `get_radiobutton` (`:664`), `dynamic_notification*` (`:843-876`), `field`/`field_placeholder_persist`/`set_growable` (`:42-72`). Progress functions also exist.
21. Xtask flags `--no-timestamp`, `--gdb-stub`, `--kernel-feature`, `--loader-feature`, `--swap`, `--lkey/--kkey`, `--feature xous/lang-*`: `xtask/src/main.rs:66,166,206-235,1177`. GDB ports 3333 (SoC) and 3334 (EC): `emulation/betrusted.resc:59,130`.
22. Renode flash backing file `tools/pddb-images/renode.bin`: `emulation/betrusted.resc:87`; `emulation/xous-release.resc` and `tools/usb_update.py` exist.
23. Status bar height = 2 x Cjk glyph height (`services/gam/src/main.rs:74-89`) and 500 ms keyboard hold delay (`services/keyboard/src/main.rs:183`); keyboard `esc_match` at `main.rs:1142`.
24. Book-aligned IPC basics in `architecture.md`: opcode enum with `FromPrimitive`/`ToPrimitive`, `receive_message` + `msg.body.id()` loop, scalar unpack macros, blocking scalar + `return_scalar` ack, `xous::connect`/`disconnect` unsafe (Book ch07-00..04; `xous-rs/src/syscall.rs:1313`). Also "APIs that do not exist": `Ticktimer::set_alarm`, `pddb::PddbError`, `PddbKey::set_len` have no definitions in source (grep over ticktimer api, pddb).

## Not verified (UNVERIFIED)

- `xtask compile-apps` and `--feature <crate>/<feature>` for per-app features (not grepped in xtask).
- Book chapters not fetched: ch07-07 (forwarding), ch07-08 (performance), ch06-01 (testing crates), ch04 (Renode), ch03-04 (GDB). Renode/PDDB timing claims in `discoveries.md:197-200` (6 min format, 45 s mount) and Renode scripting behavior are runtime facts not checkable from docs or source here.
- `agents/networking.md`, `agents/encoding.md`, `agents/graphics.md` drawing API details beyond modals, `agents/system.md` (COM, power, WiFi) and `agents/randomness.md` trng specifics were only skimmed, not line-checked against `services/com`, `services/net`, `services/trng`.
- `STANDARDS.md` font sizes (Regular 15px) and screen-layout pixel rows (nominal).
- Whether any upstream app hooks susres (G7) and whether hosted builds truly need the Xous sysroot (T5).
