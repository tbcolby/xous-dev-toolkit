# Precursor portfolio audit — slice `security`

Apps: keygen, otp, cipher (pkg `cipher-workshop`), contacts, budget, planner, qrcode (pkg `qrcode-app`), barcode.
Method: read every `.rs` file in the eight repos; cross-checked against `xous-core` HEAD 31fbe23 (gam, pddb, trng, keyboard); read the hosted-mode compiler output in `lint/`; and ran the two pure-logic encoders on the host against reference implementations (harness in `SCRATCH/harness_sec/`, nothing in the source repos was modified).

Paths below are relative to `SCRATCH/precursor/precursor-<app>/`. "Verified" means read in code and, where stated, reproduced on the host. "Suspected" means inferred from code but not run on a device.

## Headline findings

| # | Sev | App(s) | Finding |
|---|-----|--------|---------|
| 1 | Critical | barcode | Code 128 pattern table is wrong for 49 of 106 symbols (values 36–38 and 60–105, including START_B, START_C, CODE_B, CODE_C) and the stop pattern is wrong. Every Code 128 barcode the app draws is invalid. Reproduced against reportlab: 95/95 single-character inputs mismatch. |
| 2 | Critical | qrcode | QR versions 7–10 are emitted without the mandatory 18-bit version-information blocks, so data is placed into 36 reserved modules. Reproduced: 45/45 V7–V10 symbols differ from the reference under every mask; 130/130 V1–V6 symbols match exactly. |
| 3 | High (fail-open) | keygen, otp | `Rng::u32()` is `trng.get_u32().unwrap_or(0)`. Any TRNG IPC error silently yields zeros: an all-zero one-time pad (ciphertext = plaintext) or a constant "password" shown with an "EXCELLENT" rating. |
| 4 | High | keygen, otp, cipher, planner | Arrow keys are matched as Apple PUA codes U+F700–F703, which Xous never sends (it sends U+2190–2193; the toolkit README explicitly forbids F700). GAM also swallows the Menu key `∴` before apps see it. Result: cursors never move and several screens have no exit. |
| 5 | High | all 8 | PDDB handling defeats the plausible-deniability claims: state is loaded once at boot, cached forever, and written back with `basis = None` on every focus loss. After a basis lock, hidden-basis data stays on screen and is rewritten into the still-open basis; after a basis unlock, the stale cache overwrites the hidden copy. |
| 6 | High | otp | Pad reuse is not prevented on the decrypt path, the "used" flag is not confirmed as persisted, and only the first 32 bytes of a pad can ever be displayed, so a pad cannot be shared with a correspondent. |
| 7 | High | barcode | Code 39 patterns for space, `$`, `/`, `+`, `%` are rotated/duplicated: each scans as a different valid character (space reads as `$`). Silent data corruption. Reproduced against reportlab. |
| 8 | High | all 8 | None of the eight crates can have compiled as published. Beyond upstream drift there are genuine errors: undeclared `graphics_server` dependency (all), a type error in keygen, a borrow error in planner, a private-method call in contacts, missing `use core::fmt::Write` and `extern crate alloc` in four `ui.rs` files, and a `PddbKey::set_len` call that does not exist in this tree. |

No secrets are written to logs in any of the eight apps (checked every `log::` call). No app zeroizes anything (zero occurrences of `zeroize`/`volatile` in the slice). There are no unit tests in any of the eight repos.

---

## 1. keygen — "Key Ceremony"

**Verdict:** generator logic is mostly sound and the rejection sampling is unbiased, but the RNG wrapper fails open, the app cannot be navigated on real hardware, one generator never compiled, and saved keys longer than 20 characters cannot be read back. **Health: 3/10.**

### Bugs

| Sev | Location | Description | Failure scenario | Fix |
|-----|----------|-------------|------------------|-----|
| High | `src/rng.rs:18` | `get_u32().unwrap_or(0)` hides TRNG failure. | TRNG server errors once: `bytes()` returns zeros, `range()` returns 0 forever. Hex key is `0000…`, PIN is `000000`, password is a shuffle of `aA0!aaaa…`, all displayed with the normal entropy badge. | Return `Result` and propagate; show an error screen and refuse to generate. Consider `fill_buf` plus a health-test check (`get_health_tests`). |
| High | `src/app.rs:18-21` | Arrow constants are U+F700–F703. | Up/Down never match, so only "Password" can be selected. Left/Right never match, so length is fixed at the default. | Use `'↑' '↓' '←' '→'` (U+2191/2193/2190/2192). |
| High | `src/app.rs:126,153,179` with `xous-core/services/gam/src/contexts.rs:741` | GAM raises the main menu on `∴` and does not forward it to an App-layout context. `Configure`, `Result` and `Saved` exit only via Menu or the dead Left key. | Press Enter twice: the app is stuck on `Result` until reboot. Press `S` on the first screen: stuck on `Saved`. | Give every screen a deliverable back key (Backspace, `q`, or Left once fixed). Do not use `∴` for in-app navigation. |
| High | `src/generators.rs:265` | `words.push(rng.pick(WORDS))` infers `T = str`; E0277/E0308. Not drift. | Crate does not compile on any xous-core version. | `words.push(*rng.pick(WORDS))`. |
| High | `src/ui.rs:175-185` | Saved list shows only `value[..20]` + `...`; there is no detail view of the value. | Save a 64-hex-char key or a 6-word passphrase: it can never be read back. "Save to vault" is write-only for most types. | Add a view screen with wrapped full value. |
| Medium | `src/storage.rs:45-55,57-61` | Write errors are discarded; a failed or unparsable read becomes an empty list, and the next save overwrites the key. | One corrupt byte or a short write loses every saved key silently. | Return `Result`; on parse failure keep the raw key and refuse to overwrite. |
| Medium | `src/storage.rs:52` | `handle.set_len()` does not exist on `PddbKey`. If the line is simply deleted to get a build, a shorter rewrite leaves a stale tail, JSON parsing fails with trailing characters, and the list loads as empty. | Delete one saved key, reboot: all saved keys gone. | Delete the key and rewrite it (pattern used in `apps/vault/src/storage.rs:262-271`), or store a length prefix. |
| Medium | `src/main.rs:105-108`, `src/app.rs:80-84` | Unconditional full rewrite plus `pddb.sync()` on every focus loss. | Flash wear and a multi-hundred-millisecond stall on each app switch; it is also the trigger for cross-cutting finding X1. | Track a dirty flag; save only on change. |
| Low | `src/generators.rs:108-188` | Word list has 631 entries, 630 unique (`vast` twice, lines 179 and 182). | Slightly non-uniform; 9.30 bits/word. The displayed 9 bits/word is conservative, so no overstatement. | Deduplicate; consider the 7776-word EFF list or 2048-word BIP-39 list (gam already exposes `bytes_to_bip39`). |
| Low | `src/app.rs:161-171` | `S` on the result screen appends every time with no feedback. | Three presses store three copies. | Show "Saved", dedupe. |
| Low | `src/generators.rs:191-212` | `estimate_entropy` is dead code and its comment says "~72 possible chars" where the pool is 88. | None at runtime. | Remove. |
| Low | `src/generators.rs:236-247` | Forcing one character from each class slightly reduces entropy versus the displayed figure. | The integer floor (6 bits/char against 6.46 actual) more than covers it. | Document, or sample uniformly and reject. |

### Security notes
- `range()` (`rng.rs:21-30`) is correct rejection sampling: the threshold is a multiple of `max`, so no modulo bias. `shuffle` is a correct Fisher–Yates.
- Generated values live in `String`s that are cloned, replaced and dropped without wiping (`app.rs:142,158,167`); the JSON buffer in `storage.rs:64` likewise.
- Saved keys are a plaintext JSON array in dict `keygen.vault`, with no label, timestamp or purpose. PDDB encryption is the only protection.
- README: "all from quantum noise, not algorithms". The Precursor TRNG is avalanche-diode and ring-oscillator noise conditioned through a ChaCha CSPRNG in the trng server. "Hardware TRNG" is accurate; "quantum, not algorithms" is not.
- The result stays on screen indefinitely; there is no timeout or clear-on-background.

### Upstream drift
`i16` coordinates (current `Point` is `isize`), `xous::destroy_server` now `unsafe`, crate pins 0.9.69 vs 0.9.71. Separately, `ui.rs` uses `gam::Canvas`, `Gam::draw_textview`, `tv::TextView::new(bounds, text)` builder chains, `Rectangle::style()` and a 4-argument `draw_line`. None exist in this tree, and the four apps that were written against the real API (contacts, budget, qrcode, barcode) use `Gid`, `post_textview` and `TextView::new(canvas, bounds)`. Suspected: this `ui.rs` was never compiled against any xous-core. Fix pattern: port to the contacts-style drawing helpers.

### Improvements
1. Fail closed on RNG error; add a TRNG health indicator to the ceremony screen.
2. Fix keys and navigation; add a full-value view and a label field for saved keys.
3. Offer BIP-39 output via gam's existing helpers instead of a home-made list.
4. Clear the result on background and after a timeout; wipe buffers on drop.
5. Unit-test `range`, base64 and hex against vectors with a mock RNG.

---

## 2. otp — "One-Time Pad"

**Verdict:** the XOR and hex code is correct, but the properties that make a one-time pad meaningful (fresh randomness, single use, a shared copy of the pad) are not enforced or not achievable, and the UI cannot be driven on hardware. This is a one-time pad, not HOTP/TOTP, so RFC 4226/6238 does not apply. **Health: 3/10.**

### Bugs

| Sev | Location | Description | Failure scenario | Fix |
|-----|----------|-------------|------------------|-----|
| High | `src/rng.rs:18` | Same `unwrap_or(0)` as keygen. | TRNG error during `bytes(64)`: the pad is partly or wholly zero, and the "ciphertext" is the plaintext in hex. Nothing indicates a problem. | Propagate the error; never create a pad on failure. |
| High | `src/app.rs:159-167`, `295-301` | Decrypt accepts any pad and never marks it used. | Decrypt an incoming message with pad 3 (still FRESH), then encrypt a reply with pad 3: a two-time pad, and XOR of the two ciphertexts reveals XOR of the plaintexts. | Mark used on decrypt too, or model direction explicitly (separate send and receive pads). |
| High | `src/ui.rs:204`, `src/pad.rs:126-140` | `ViewPad` shows `format_key_preview(&pad.key, 32)`. There is no export. | Default pads are 64 bytes; the second half can never be seen, so the pad cannot be given to a correspondent. The tool can only encrypt to itself, on a device whose storage is already encrypted. | Paged full-key view, or pad export via QR (the sibling app). |
| High | `src/app.rs:21-24` | F700 arrow codes. | Cannot move the pad cursor (only the newest pad is reachable), cannot reach the Length field (every pad is 64 bytes). | Use U+2190–2193. |
| High | `src/app.rs:181,210,255,289` | Menu is the only cancel. | Press `E` on a pad: the only way forward is to type something and press Enter, which burns the pad. Press `G` by mistake: must create a pad to leave. | Add Backspace-on-empty or another deliverable cancel. |
| Medium | `src/app.rs:267-269`, `src/storage.rs:40-50` | `used = true` is saved with errors ignored, then the ciphertext is shown. | The write fails (full flash, IPC error): ciphertext has been disclosed, but after reboot the pad is FRESH again. | Check the write and sync result before revealing ciphertext. |
| Medium | `src/ui.rs:88-102,258-267` | `draw_wrapped` stops at the footer. | A message over roughly 360 bytes produces hex that is cut off on screen; the remainder is unrecoverable and the pad is spent. | Page the result, or cap the pad length to what can be shown. |
| Medium | `src/app.rs:331-342` | "Destroy" removes the entry and rewrites JSON. README says "securely erase key material". | The key bytes remain in freed heap and in superseded PDDB pages until garbage-collected. | Overwrite the `Vec<u8>` before drop; soften the README claim; call `sync_cleanup`. |
| Medium | `src/storage.rs:52-56` | Unparsable pads key loads as an empty list and is then overwritten. | All pads lost silently. | As keygen. |
| Low | `src/pad.rs:21-27` | `length` duplicates `key.len()`. | If they disagree after deserialization, typing is limited by `length` while `xor_apply` checks `key.len()`; Enter then does nothing. | Drop the field. |
| Low | `src/pad.rs:20` | `#[derive(Debug)]` on `Pad` includes the key. | Not logged today; a future `{:?}` would leak it. | Manual `Debug` that redacts. |
| Low | `src/ui.rs` pad list | No scrolling; rows past the footer are not drawn. | More than about 18 pads: the cursor is off-screen. | Scroll window. |

### Security notes
- XOR, hex encode/decode and length enforcement are correct. Encryption consumes from offset 0 and marks the whole pad used, which is safe though wasteful.
- An OTP gives no integrity. Ciphertext is malleable bit-for-bit, and the README's "mathematically proven to be unbreakable" should carry that caveat plus the fact that message length is revealed.
- Plaintext (`input_text`) and results persist in memory after use; nothing is wiped.
- Pads are stored as JSON arrays of decimal numbers in `otp.vault`, about four bytes of flash per key byte.

### Upstream drift
Identical to keygen (same template `main.rs`, same suspected-never-compiled `ui.rs` API, `set_len`).

### Improvements
1. Decide the threat model: two-party (needs export and paired send/receive pads) or local-only (then say so).
2. Per-pad consumption offset so a 512-byte pad serves many short messages without reuse.
3. Fail closed on RNG and on persistence errors.
4. Add an optional one-time MAC.
5. Host unit tests for `pad.rs` (it has no Xous dependencies).

---

## 3. cipher — "Cipher Workshop"

**Verdict:** honestly presented as classical, educational ciphers, so there is no "homebrew crypto sold as secure" problem. The cipher math is right for valid keys, but input validation is missing and, because of the key-code bugs, the only operation reachable on hardware is Caesar with shift 3, after which the app is stuck. **Health: 3/10.**

### Bugs

| Sev | Location | Description | Failure scenario | Fix |
|-----|----------|-------------|------------------|-----|
| High | `src/app.rs:19-22` | F700 arrow codes. | Cipher cursor never moves (Caesar only); shift and rail count cannot be adjusted with arrows. | Use U+2190–2193. |
| High | `src/app.rs:324` | The output screen exits only on Menu or Left. | After the first encryption the app shows that result until reboot. `F` then Enter returns to the same screen. | Add a deliverable back key. |
| High | `src/app.rs:264-270` | Digit entry computes `shift*10 + digit` and accepts only 1–25. | From the default 3, every digit gives 30–39 and is rejected. With arrows dead, Caesar is locked at 3. | Keep a separate digit buffer; replace rather than append when out of range. |
| Medium | `src/app.rs:306-315`, `src/ciphers.rs:135,141,157` | Vigenere key accepts any printable character; `to_upper(c) as u8 - b'A'` underflows for non-letters. | Key `my key` or `k3y`: debug and hosted builds panic on overflow; release builds wrap and produce output that `vigenere_decrypt` does not invert. | Filter key entry to A–Z; skip or reject other characters in the cipher. |
| Medium | `src/ciphers.rs:165,190` | Substitution with fewer than 26 characters returns the input unchanged. | Enter a 10-letter alphabet: "ciphertext" equals plaintext, presented as encrypted. | Require exactly 26; show an error. |
| Medium | `src/ciphers.rs:193-208` | Alphabet is not checked to be a permutation. | Duplicates or symbols: letters with no preimage decrypt to NUL (`inv` stays 0); encryption is not invertible. | Validate 26 distinct letters. |
| Low | `src/app.rs:142-163` | Saved messages keep plaintext, ciphertext and key together. | Acceptable for a teaching tool; contradicts any reading of "your encrypted messages stay on your device" as protection. | Document. |
| Low | `src/ui.rs` saved list | 20/24-character previews only; no full view. | Saved work cannot be read back in full. | Detail screen. |
| Low | `src/app.rs:359-363` | Frequency scroll range is 0–12 with an 18-row window, though all 26 rows fit at 16 px. | Unneeded scrolling. | Draw all 26. |

### Upstream drift
Same as keygen. Additionally the package is named `cipher`, which collides with the RustCrypto `cipher` crate in the xous-core workspace, while the code references `gam::APP_NAME_CIPHER_WORKSHOP`. The manifest key must therefore be `cipher-workshop`, which does not match the package name; the README documents neither. Rename the package to `cipher-workshop`.

### Improvements
1. Fix keys and navigation first; nothing else is reachable.
2. Validate keys at entry time with visible errors.
3. `ciphers.rs` is pure: add round-trip unit tests for every cipher and run them on the host.
4. Add a short "why this is breakable" note per cipher; the frequency chart already supports that story.

---

## 4. contacts — "Contact Book"

**Verdict:** the best-structured app in the slice (correct key codes, real GAM API, per-record storage, working scroll and search). It still does not compile as published, and its "hardware vault with plausible deniability" claim is not backed by any basis handling. **Health: 5/10.**

### Bugs

| Sev | Location | Description | Failure scenario | Fix |
|-----|----------|-------------|------------------|-----|
| High | `src/ui.rs:401` vs `src/app.rs:171` | `selected_contact_index` is private but called from `ui.rs` (E0624). Not drift. | Does not compile. | `pub fn`. |
| High | `src/ui.rs` (8 `write!` sites, 10 `alloc::` sites) | No `use core::fmt::Write`; `alloc::format!` without `extern crate alloc`. Not drift. | Does not compile. | Add both lines. |
| High | `src/storage.rs:28-49`, `src/main.rs:108-113` | No basis awareness; see X1. | Contacts created while a secret basis is open remain visible after it is locked, and the next save writes an index containing their IDs into the open basis. | See X1. |
| Medium | `src/storage.rs:58-68` | `next_id` falls back to 1 if `config` is missing or unparsable. | `config` is lost while contacts exist: the next new contact is written to `contact.1`, overwriting an existing record. | Derive `max(id)+1` from loaded contacts. |
| Medium | `src/app.rs:234` | `q` on the list returns `false`, and `main` terminates the process. | Suspected: the app cannot be relaunched from the menu until reboot, and GAM keeps a context whose listener is gone. No upstream app exits this way. | Remove quit; let the Menu key switch apps. |
| Medium | `src/storage.rs:46` | `set_len` missing; see keygen row. | If removed naively, editing a contact to a shorter value makes that record unparsable, and `load_all_contacts` silently skips it (`storage.rs:89-93`). | Delete and rewrite. |
| Low | `src/app.rs:393` | With an empty query, `q` exits search. | Cannot search for names starting with Q without typing another letter first. | Use Backspace-on-empty to exit. |
| Low | `src/app.rs:355-359` | Saving with an empty name discards all other fields without a message. | Phone and notes typed, name forgotten: lost. | Stay on the form and show an error. |
| Low | `src/app.rs:273-280` | Both branches of the Enter handler are identical; the comment says "move to next" but it does not. | Extra keystrokes per field. | Advance the field cursor. |
| Low | `src/app.rs:412-427` | Delete-target selection has redundant conditions that all reduce to "use the list cursor". | Works today, fragile. | Store the target ID when entering `ConfirmDelete`. |
| Low | `src/contact.rs:123-131` | Category sort uses case-sensitive name order, alphabetical sort is case-insensitive. | Inconsistent ordering between modes. | Share the comparator. |

### Security notes
- README and DESIGN state "hardware vault", "plausible deniability", "contacts can exist in basis sets that are invisible without the correct password". The code never names a basis, never lists bases, never reacts to basis changes. Whatever deniability exists is PDDB's default behaviour, and the cache-and-rewrite pattern in X1 works against it.
- Dictionary name `contacts.data` and key names `contact.<id>` are fixed and reveal record count to anyone who can open the basis. That is acceptable, but it is not deniable within a basis.
- No wiping of deleted contacts in memory.

### Upstream drift
`graphics_server::` paths (types now come from `ux_api::minigfx` and `gam` re-exports; the dependency is also undeclared in `Cargo.toml`), `destroy_server` unsafe, version pins. Fix pattern: add `ux-api`/`blitstr2` dependencies as `apps/hello` does and replace the `graphics_server::` prefix.

### Improvements
1. Basis-aware storage with a visible indicator of which basis a contact lives in.
2. Search across phone, email and notes, not just name.
3. Use the measured canvas size (see X4); the 18-row list assumes 536 px.
4. Export a contact as a vCard QR through the QR encoder.
5. Unit tests for `matches` and both sorts.

---

## 5. budget — "Budget Ledger"

**Verdict:** simple and mostly correct, with integer-cent arithmetic done properly in the summary. It has the same genuine compile errors as contacts, an overflow in amount parsing, and no dates on transactions. **Health: 5/10.**

### Bugs

| Sev | Location | Description | Failure scenario | Fix |
|-----|----------|-------------|------------------|-----|
| High | `src/ui.rs` (7 `write!`, 12 `alloc::`) | Missing `use core::fmt::Write` and `extern crate alloc`. Not drift. | Does not compile. | Add both. |
| Medium | `src/app.rs:321-335` | Up to 10 digits are accepted; `dollars * 100 + cents` is `u32`. | `50000000` (eight digits) wraps to 705,032,704 cents in release and panics in debug. Ten digits above 4,294,967,295 fail `parse` and become 0. | Parse to `u64`, cap, and reject on overflow. |
| Medium | `src/storage.rs:46-51` | Unparsable ledger loads as empty and is then overwritten. | Total silent data loss on one bad read. | Refuse to overwrite on parse failure. |
| Medium | `src/storage.rs:41` | `set_len` missing. | If removed naively: delete one transaction, reboot, ledger is empty. | Delete and rewrite. |
| Medium | `src/app.rs:151` | `q` on the summary terminates the process. | Suspected: app dead until reboot. | Remove. |
| Medium | `src/ledger.rs:106-112` | Transactions have no date or timestamp. | A ledger cannot be reviewed by period; "recent" is only insertion order. | Add a timestamp from the system clock. |
| Low | `src/app.rs:339-346` | A zero amount is dropped without a message. | User believes it was saved. | Show an error. |
| Low | `src/ledger.rs:79-92` | `next()` cycles all ten categories for both types; `expense_categories`/`income_categories` are unused. | Income tagged "Housing". | Cycle within the type's list. |
| Low | `src/app.rs:201-209` | IDs are consumed on cancelled adds. | Cosmetic. | Allocate on save. |
| Low | `src/storage.rs:53-56` | Whole ledger is one JSON key, rewritten on every change and every focus loss. | Thousands of entries: growing heap spikes and slow saves. | Chunk by month or one key per transaction. |

### Security notes
Claims are modest ("encrypted at rest") and accurate as far as PDDB goes. X1 applies.

### Upstream drift
Same as contacts.

### Improvements
1. Dates, edit-in-place, and a per-category or per-month summary.
2. Currency setting rather than a hard-coded `$`.
3. `ledger.rs` is pure: unit-test `summarize`, `format_cents`, and extract and test `parse_amount`.

---

## 6. planner — "Day Planner"

**Verdict:** date arithmetic is correct, but the app always opens on a hard-coded date, has a real borrow-check error, and with the F700 key codes can only ever reach the 1st of a month. **Health: 3/10.**

### Bugs

| Sev | Location | Description | Failure scenario | Fix |
|-----|----------|-------------|------------------|-----|
| High | `src/app.rs:226` | `events_for_date()` borrows `self`, then `self.form_title` is assigned (E0506). Not drift. | Does not compile. | Clone the selected event before assigning. |
| High | `src/app.rs:21-24` | F700 arrow codes. | No day navigation; in month view the cursor cannot move, so `[`/`]` plus Enter only ever selects day 1. Hour, minute and priority fields are unreachable: every event is 9:00 AM, Normal. Task and event cursors never move. | Use U+2190–2193. |
| High | `src/main.rs:129-138` | `epoch_to_date` ignores its input and returns 2026-01-01. | The planner always opens on 1 January 2026. The comment says there is no accessible RTC, but Xous `std::time::SystemTime` and the status app's local-time path provide one. | Read system time, fall back to the last-viewed date. |
| Medium | `src/app.rs:311,333,364,392,453` | Menu is the only cancel in the event and task forms. | Event form: Enter with an empty title happens to cancel. Task form: same. Otherwise stuck. | Deliverable cancel key. |
| Medium | `src/storage.rs:52,57-72` | `set_len` missing; unparsable data loads as empty and is overwritten. | As budget. | As budget. |
| Low | `src/app.rs:278-281` | After toggling a task, `sort_tasks` moves it but the cursor stays on the old index. | Enter twice toggles two different tasks. | Re-find the task by ID after sorting. |
| Low | `src/app.rs:502-507` | Cancelling a delete always returns to `DayView`; the `match self.state { _ => … }` is a no-op. | Cancelling a task delete drops you out of the task list. | Remember the origin state. |
| Low | `src/app.rs:515` | In month view Menu and Enter both commit the cursor date. | "Back" changes the date. | Separate them. |
| Low | `src/app.rs:428-446` | Editing to an empty title keeps the old title silently. | Confusing. | Validate. |
| Low | `src/planner.rs:76-90` | `weekday()` (Zeller) returns 0 = Saturday despite its doc comment; unused. | Latent trap. | Delete; `day_of_week()` is correct. |
| Low | `src/ui.rs:360` | "Today" box marks the selected date, not the real date. | Misleading. | Track real today separately. |

### Security notes
"Encrypted day planner" rests entirely on PDDB. X1 applies. Nothing sensitive is logged.

### Upstream drift
Same suspected-never-compiled `ui.rs` API as keygen. `ticktimer` is instantiated only to feed the ignored `epoch_to_date` argument.

### Improvements
1. Real clock, then key fixes; both are prerequisites for any use.
2. Recurring events and an agenda view.
3. `planner.rs` is pure: test leap years, month rollover and `day_of_week` against known dates.

---

## 7. qrcode — "QR Code Generator"

**Verdict:** the from-scratch encoder is well made and exactly right for versions 1–6, but versions 7–10 are invalid, deletion leaves payloads in PDDB, and rendering makes one blocking IPC call per dark module. **Health: 5/10.**

### Encoder verification
Host harness (`SCRATCH/harness_sec/qr_main.rs`, `qr_cmp.py`) compared the module matrix with python-qrcode at the same version, EC level and each of the eight masks, over numeric, alphanumeric and byte inputs of 1–271 characters at L/M/Q/H:

| Version | Cases | Exact match |
|---------|-------|-------------|
| 1–6 | 130 | 130 |
| 7–10 | 45 | 0 |

By inspection the capacity, block, EC-codeword and alignment tables are all correct for V1–V10, as are GF(256) arithmetic (0x11D), Reed–Solomon, interleaving, format-info BCH and placement, the eight mask formulas, the dark module, and character-count widths. The single defect is the missing version information.

### Bugs

| Sev | Location | Description | Failure scenario | Fix |
|-----|----------|-------------|------------------|-----|
| Critical | `src/qr_encode.rs:866-938` (`encode`), `592-615` (`reserve_format_info`) | No version-information encoding or reservation for V≥7. | Any payload over 134 bytes at L, 106 at M, 74 at Q or 58 at H selects V7+. The two 6×3 blocks hold data instead of BCH(18,6) version bits and all later codewords are displaced. The symbol does not decode. A `WIFI:` string carrying a 63-character passphrase (about 90 bytes) lands here at EC-Q or EC-H. | Reserve both blocks for V≥7 and write the 18-bit value (generator 0x1F25). Until then cap at V6 and report "too long". |
| High | `src/storage.rs:146-192`, `src/app.rs:381` | Deleting a saved code rewrites the index but never deletes `code.<name>`. | A "deleted" WiFi password or wallet key remains in PDDB indefinitely, readable by anything that can open the dict. | Call `pddb.delete_key` for removed entries. |
| Medium | `src/storage.rs:168`, `src/app.rs:328-338` | The save name is the PDDB key; duplicates are not checked. | Save "wifi" twice with different text: both list entries load the second text; the first payload is lost. | Reject duplicates or key by ID. |
| Medium | `src/ui.rs:321-338` | One `gam.draw_rectangle` IPC per dark module. | V6 has about 800 dark modules, V10 about 1,600; each redraw (including every focus regain) issues that many blocking calls. Suspected to take seconds on device. | Merge horizontal runs, batch with `draw_list`, or blit a `Bitmap`. |
| Medium | `src/qr_encode.rs:277-285` | `gf_mul` rebuilds the 256-entry log and 512-entry exp tables on every call. | Thousands of table rebuilds per symbol on a 100 MHz core. | Build once (`const` tables or a struct). |
| Medium | `src/qr_encode.rs:749,758` | Penalty rule 3 allocates a `Vec<bool>` per window position: roughly 5,000 allocations per mask at V10, times 8 masks. | Slow encode and heap churn. | Compare in place. |
| Medium | `src/app.rs:235-246` | Template and clear shortcuts match U+F001–F004; the keyboard sends 0x11–0x14 for F1–F4 (`graphics-server/src/backend/minifb.rs:357` in hosted mode; the hardware layouts in `keyboard/src/mappings/` use the same 0x11–0x14). | F1–F4 do nothing, although README and the help screen advertise them. | Match `'\u{11}'..='\u{14}'`. |
| Medium | `src/app.rs:260-271` | Encode failure only logs a warning. | At EC-H the limit is 119 bytes but input allows 271: Enter appears dead. | Show capacity for the current EC level and an error. |
| Medium | `src/app.rs:288-304`, `src/ui.rs:296-304` | Manual module size up to 8 px is not clamped to the canvas. | V6 at 8 px is 392 px wide: negative offset, clipped, unscannable. | Clamp to `width / (size + 8)`. |
| Medium | `src/app.rs:204` | `q` on the main menu terminates the process. | Suspected: app dead until reboot. | Remove. |
| Low | `src/app.rs:220-229` | `q` with empty input leaves the screen. | Text cannot start with `q` or `Q`. | Different exit key. |
| Low | `src/app.rs:373-376` | Loading a saved code changes the global EC setting, which is persisted on the next background. | Setting drifts. | Use the code's EC level locally. |
| Low | `src/main.rs:107` | `log::debug!("Key: …")` for every keystroke. | Suppressed at the current Info level; if someone raises the level, typed secrets go to the serial log. | Remove. |
| Low | `src/storage.rs:46-49` | `module_size` loaded unchecked (`u64 as u8`). | Corrupt config gives odd sizes. | Clamp. |

### Security notes
- README frames QR as "the most secure channel between two devices". An optical code is readable by any camera in line of sight and stays on a non-volatile-looking display until redrawn; this deserves a caveat and a clear-on-background option.
- README claims saved codes get plausible deniability; X1 applies, and the delete bug above is the more concrete problem.
- Byte mode emits UTF-8 without an ECI header. This is common practice and input is limited to ASCII, so no issue today.

### Upstream drift
`graphics_server::` paths and undeclared dependency, missing `use core::fmt::Write`, three `set_len` calls, `destroy_server` unsafe. README tells the user to add manifest key `qrcode` with context name "QR Code", but the code references `gam::APP_NAME_QRCODE_APP`, which requires key `qrcode-app`. Following the README produces a compile error.

### Improvements
1. Version info for V7–V10 (about 40 lines) and a regression test over all 40 version/EC combinations; the harness in `SCRATCH/harness_sec/` is a starting point.
2. Run-length rendering and static GF tables.
3. Real key deletion and duplicate-name handling.
4. Mixed-mode segmentation is not needed; a live "N of M bytes at EC-x" counter is.
5. Move `qr_encode.rs` into a shared `no_std` crate with tests; contacts and otp could use it.

---

## 8. barcode — "Barcode Generator"

**Verdict:** EAN-13 and UPC-A are correct. Code 128 is entirely broken and Code 39 mis-encodes five characters, and those two are what auto-detect chooses for all non-numeric input. Anything longer than about nine characters is silently clipped at the default width. **Health: 3/10.**

### Encoder verification
Host harness (`SCRATCH/harness_sec/bc_main.rs`, `bc_cmp.py`) against reportlab's Code 128 and Code 39 encoders and its EAN tables:

| Symbology | Cases | Result |
|-----------|-------|--------|
| EAN-13 / UPC-A | 600 random | 600 match, including check digits |
| Code 39 | 43 characters | 38 match; space, `$`, `/`, `+`, `%` wrong |
| Code 128 | 95 set-B characters, 100 set-C pairs, 300 random | 0 match |

### Bugs

| Sev | Location | Description | Failure scenario | Fix |
|-----|----------|-------------|------------------|-----|
| Critical | `src/barcode_encode.rs:153-155,177-222` | 49 of 106 pattern rows are wrong. 43 of the 45 rows from 60 to 104 sum to 10 modules instead of 11; rows 38 and 74 contain a zero width and are commented "placeholder"; START_B (104) is `1,2,1,1,3,2` instead of `2,1,1,2,1,4`; START_C, CODE_B, CODE_C are wrong too. | Every Code 128 symbol starts with an invalid start character. Independently, `D`, `E`, `F`, backslash through `~` (all lowercase letters) and about half of all checksum values use wrong patterns. Nothing scans. | Replace the table with the ISO/IEC 15417 values and add a test that every row sums to 11 and all rows are distinct. |
| Critical | `src/barcode_encode.rs:329-331` | Stop pattern is `1100011101101` (widths 2,3,3,1,2,1,1); correct is `1100011101011` (2,3,3,1,1,1,2). | Even with a correct table, no symbol terminates validly. | Fix the literal. |
| High | `src/barcode_encode.rs:397-401` | Code 39 rows: space holds the `$` pattern, `$` holds `/`, `/` holds `+`, `+` holds `%`, `%` holds `$` again (comment: "same as space visually — simplified"). | `HELLO WORLD` scans as `HELLO$WORLD`; `A/B` scans as `A+B`. Valid symbols with wrong content. | space `0,1,1,0,0,0,1,0,0`; `$` `0,1,0,1,0,1,0,0,0`; `/` `0,1,0,1,0,0,0,1,0`; `+` `0,1,0,0,0,1,0,1,0`; `%` `0,0,0,1,0,1,0,1,0`. |
| High | `src/ui.rs:245-253` | A barcode wider than the screen is drawn from x=4 and cut off with `break`. | Default bar width is 2 px, leaving 164 modules: about 9 Code 128 or 7 Code 39 characters. Input allows 80. Longer codes render truncated with no warning. | Auto-reduce bar width, refuse when it cannot fit at 1 px, and show the limit while typing. |
| High | `src/storage.rs:109-143`, `src/app.rs:366-371` | Delete never removes `code.<name>`; duplicate names overwrite each other. | Same as qrcode: "deleted" credential barcodes persist. | `delete_key`; unique names. |
| Medium | `src/barcode_encode.rs:558-562,643` | A wrong EAN-13 or UPC-A check digit is silently replaced. | A typo in a 13-digit number yields a valid barcode for a different number. | Reject with a message; auto-append only when the digit is omitted. |
| Medium | `src/app.rs:217-229` | F1–F4 matched as U+F001–F004. | Format shortcuts never fire. | Use 0x11–0x14. |
| Medium | `src/app.rs:246-256` | Encode failure only logs. | 11-digit input with EAN selected: Enter appears dead. | Show the reason. |
| Medium | `src/app.rs:197` | Terminates the process. | Suspected: app dead until reboot. | Remove. |
| Low | `src/barcode_encode.rs:101-110` vs `255` | `is_valid` allows 0–127 for Code 128, `encode_code128` allows 32–126. | Inconsistent validation. | Share one predicate. |
| Low | `src/barcode_encode.rs:567,607` | EAN quiet zones are 9 modules each side; the spec asks for 11 left and 7 right. | Marginal scanners may struggle at the left edge. | 11 / 7. |
| Low | `src/main.rs:85` | Keystroke `log::debug!`. | As qrcode. | Remove. |

### Security notes
README markets "credential barcodes — saved and encrypted at rest". X1 and the delete bug apply. A 1-D barcode of a credential is trivially photographed; a caveat would be honest.

### Upstream drift
Same as qrcode: `graphics_server::` paths and undeclared dependency, 20 `write!` sites without the trait import, three `set_len` calls.

### Improvements
1. Correct tables plus table-invariant tests; the comments in the file ("placeholder", "not used directly below") show the table was known to be unfinished.
2. Fit-to-width logic and a live capacity indicator.
3. Code 128 set-C optimisation for digit runs is fine as written; add set A only if control characters are wanted.
4. Share storage and list UI with qrcode; the two apps are near-copies.

---

## Cross-cutting findings

### X1. PDDB basis handling undermines the deniability claims (all 8; High)
- Every `Storage` uses `pddb.get(dict, key, None, …)` and never passes a `key_changed_cb`, never calls `monitor_basis`/`list_basis`, and loads once in `init_storage` at boot.
- With `basis_name = None`, PDDB writes to the most recently opened basis (`xous-core/services/pddb/src/backend/basis.rs:291-301`, `select_basis`).
- Every `main.rs` calls `app.save_state()` on each `FocusState::Background`, dirty or not. For keygen, otp, cipher, budget and planner that rewrites the full dataset; for contacts it rewrites the index; qrcode and barcode rewrite settings only (their saved codes are cached but written on save/delete).
- Consequences, from code reading and not device-tested:
  1. Open a secret basis, add data, lock the basis. The app still holds and displays that data, and its next save writes it (OTP key material, generated passwords, the contact index) into the basis that remains open.
  2. Boot, then open a secret basis that already has app data. The app does not reload; on the next focus loss it writes its boot-time cache into the secret basis, shadowing or replacing what was there.
  3. The user has no indication of which basis a record will land in.
- Fix pattern: one shared storage layer that (a) subscribes to basis changes and drops and reloads its cache, (b) writes only when dirty, (c) shows the target basis, and (d) returns `Result`.

### X2. Key handling (keygen, otp, cipher, planner; High)
- U+F700–F703 never arrive. `xous-dev-toolkit/README.md:228` already says "Do NOT use Apple PUA codes". The other four apps use the right codes.
- `∴` is consumed by GAM for App-layout contexts once the main menu is allowed, so "Menu = Back/Quit" in all eight footers and READMEs cannot work.
- F1–F4 arrive as 0x11–0x14, not U+F001–F004 (qrcode, barcode).
- Recommendation: one shared `keys` module with the correct constants and a unit test, and a rule that every screen has a non-Menu exit.

### X3. Storage robustness (all 8; Medium)
- Identical `read_key`/`write_key` copy-pasted eight times: every error is `.ok()`'d, parse failure becomes "empty", and the next save destroys the original.
- `PddbKey::set_len` does not exist in this tree; the shorter-rewrite truncation problem it was meant to solve is real and needs delete-and-rewrite.
- No format version field in any stored JSON.
- `next_id` counters stored separately from the data they index (contacts, budget, otp, planner) can reset to 1 and collide.

### X4. Canvas size (all 8; Low–Medium, Suspected)
Every app fetches `get_canvas_bounds`, logs it, and then hard-codes 336×536. A Framebuffer canvas starts below the status bar (`gam/src/layouts/framebuffer.rs:30`), so it is shorter than 536. Footers placed at y = 490–536 and anything centred on 536 are partly clipped.

### X5. Process exit (contacts, budget, qrcode, barcode; Medium, Suspected)
`q` on the top screen breaks the loop, unregisters the server and calls `terminate_process(0)`. Xous apps are started at boot and are not respawned, and no upstream app exits. The same code exists in the other four but is unreachable because it is bound to Menu.

### X6. Build and packaging (all 8)
- `graphics_server` is used in every `ui.rs` and declared in no `Cargo.toml`. On current xous-core the types live in `ux-api`/`blitstr2`.
- `ticktimer-server` is declared by all eight and used only by planner (trivially); `xous-ipc` is declared and unused by all eight; `const APP_NAME` is unused in all eight.
- `#![cfg_attr(target_os = "none", no_std)]` is inert on Xous (`target_os = "xous"`), and the code depends on `std::io` and `String` regardless.
- Only qrcode and barcode READMEs describe manifest integration, and qrcode's is wrong. cipher and qrcode need manifest keys that differ from their package names.
- No tests anywhere, although `generators.rs`, `pad.rs`, `ciphers.rs`, `contact.rs`, `ledger.rs`, `planner.rs`, `qr_encode.rs` and `barcode_encode.rs` are all pure and host-testable. The two encoder defects above were found in minutes by exactly such tests.

### X7. Secret hygiene (keygen, otp, qrcode, barcode)
- No zeroization of keys, pads, plaintext or JSON buffers.
- Secrets remain on the display when the app is backgrounded and indefinitely while foregrounded.
- `Debug` is derived on secret-bearing structs (`Pad`, `SavedKey`, `SavedCode`).
- Positive: no secret is logged anywhere today.

### X8. Duplication worth a shared crate
`main.rs` event loop (eight near-identical copies), `Storage` read/write, key constants, header/footer/text drawing helpers (two incompatible families), list-with-cursor-and-scroll, text-entry field, and the qrcode/barcode save-load-settings flow. A `precursor-app-kit` crate would remove most of findings X1–X5 in one place.

## Suggested order of work
1. barcode Code 128 table and stop pattern, Code 39 five rows; qrcode version info. Add host tests with them.
2. RNG fail-closed in keygen and otp.
3. Key constants and non-Menu exits in keygen, otp, cipher, planner.
4. Shared storage layer: `Result`, dirty flag, delete-and-rewrite, real key deletion, basis-change reload.
5. Get all eight compiling against current xous-core (dependency declarations, trait imports, the three genuine code errors), then re-verify on Renode.
6. Revise README security claims to match what the code does.
