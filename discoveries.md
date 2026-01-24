# Discoveries & API Notes

Notes accumulated while developing Precursor apps. Update this as you learn new things.

## Confirmed Patterns (Updated 2026-01-23 from Flashcards app build)

- **Do NOT depend on `ux-api` or `blitstr2` directly** — they require platform feature flags.
  Use `gam::menu::*` (re-exports `ux_api::minigfx::*`) and `gam::GlyphStyle` instead.
- `Point` coordinates are `isize`, not `i16` (important for layout arithmetic)
- `std::net::TcpListener`/`TcpStream`/`UdpSocket` work directly in apps — no `net` crate needed.
  Xous routes std::net calls to the net service via IPC automatically.
- PDDB `get()` returns a `PddbKey` implementing Read/Write/Seek — always Seek(Start(0)) before reading
- PDDB feature flags (precursor/hosted/renode) are handled by `cargo xtask`, not by app Cargo.toml
- GAM rate limits redraws to ~30fps (33ms minimum between flushes)
- `GamObjectList` is preferred over individual draw calls for atomicity
- Server names must be globally unique across all running processes
- `main() -> !` pattern is mandatory; use `xous::terminate_process(0)` to exit
- Apps must handle `FocusChange` to avoid wasting CPU when backgrounded
- TLS certificate trust is interactive - first connection to new host prompts user

## API Quirks

- `APP_NAME` in UxRegistration must match `context_name` in `apps/manifest.json`
- `SERVER_NAME` for xous-names registration is separate from `APP_NAME` (convention: `_AppName_`)
- Modal radiobuttons: Enter at item index just selects payload; Enter at items.len() confirms
- Menu key is '∴' (U+2234) - used for BOTH opening menu AND selecting items in menu
- `allow_mainmenu` is set to true automatically after PDDB mount (not PIN entry per se)
- Menu uses rawkeys with '∴' to select, '↑'/'↓' to navigate — NOT Enter!
- Arrow keys produce Unicode arrows: '→' (U+2192), '←' (U+2190), '↑' (U+2191), '↓' (U+2193)
- Do NOT use Apple PUA codes (\u{F700}-\u{F7FF}) for arrow keys in Xous apps

## Keyboard Hold Timing (Critical for Renode)

- Keyboard service has 500ms hold threshold (configurable, default 500ms)
- If press-to-release emulated time >= 500ms, the `hold` variant is produced
- Keys with `hold: None` (arrows, Home, Space) produce NOTHING when held — silently dropped!
- Keys with `hold: Some(x)` produce the hold character (e.g. 'h' hold → '+', 'a' hold → '@')
- Enter key has same character for all variants (0x0D) — always works regardless of timing
- Renode keyboard C# has two shift scan code sets:
  - `ShiftLeft`/`ShiftRight` → (3,0)/(3,9) — WRONG for Xous
  - `ShiftL`/`ShiftR` → (8,5)/(8,9) — CORRECT for Xous keyboard service

## Things That Don't Work As Expected

- Renode GUI ("XWT") does not work on macOS ARM64 portable package - use `--disable-xwt`
- Keyboard peripheral Reset desyncs the keyboard service's shift/hold/layer state
- InjectKey path only passes bottom 8 bits of characters (can't inject Unicode like '∴')
- PDDB first-boot blocks UI until init complete - but format only takes ~5 min on blank image
- Renode keyboard timing is CPU-load-dependent: idle CPU = faster emulated time = easier to trigger hold
- **Solution**: Use `pause → Press → emulation RunFor "0:0:0.001" → Release → start` to hold key for exactly 1ms emulated time (well under 500ms threshold). This works reliably for all keys.
- For text input (PIN entry): `sysbus.keyboard InjectLine "text"` injects characters + CR without hold timing issues. CR (0x0D) acts as submit in PIN/text dialogs.
- Radio dialog navigation: Down key moves cursor between options; submit only fires when cursor is at [Okay] button (index >= items.len()). Sequence: Down, Down, then CR to confirm.

## Renode PDDB Setup

- Renode keybox PIN is `a` (single character, hardcoded in `emulation/renode-keybox.bin`)
- Blank 128 MiB image at `tools/pddb-images/renode.bin` (create with `\xff` fill)
- First boot: format dialog (Down,Down,CR to confirm) → InjectLine "a" → InjectLine "" (dismiss) → InjectLine "a" (confirm) → ~6 min format → InjectLine "a" (unlock) → mount → ready
- Pre-formatted backup: `tools/pddb-images/renode-formatted.bin`
- Copy formatted version to `renode.bin` to skip re-initialization on subsequent runs

## Performance Notes

- 100MHz CPU means JSON parsing is noticeable; prefer small payloads
- PDDB sync is expensive; batch writes before calling sync()
- Network latency compounds with TLS handshake overhead

## Toolchain Notes

- Target triple: `riscv32imac-unknown-xous-elf` (userspace), `riscv32imac-unknown-none-elf` (kernel)
- Does NOT use Rust nightly - uses stable Rust with a custom sysroot
- Custom sysroot from `betrusted-io/rust` GitHub releases (match your stable Rust version)
- Install sysroot to: `~/.rustup/toolchains/stable-<host>/lib/rustlib/riscv32imac-unknown-xous-elf/`
- `cargo xtask renode-image <app>` builds the full image with the specified app
- Apps must be registered in: workspace `Cargo.toml` members + `apps/manifest.json`
- Renode provides full system emulation including network (but GUI doesn't work on macOS ARM64)

## Completed Apps

### Flashcards (App #1) — 2026-01-23
- **Repo**: https://github.com/tbcolby/precursor-flashcards
- **Features**: Multi-deck PDDB storage, TCP import (port 7878), state machine UI
- **Key deps**: `gam`, `pddb` (no direct ux-api/blitstr2)
- **Build**: `cargo xtask renode-image flashcards` — compiles clean (0 warnings)
- **Import format**: TSV with `#name:` header, pushed via `cat deck.tsv | nc <ip> 7878`
- **Lessons learned**: gam::menu::* for graphics types, Point is isize, std::net just works

### Timers (App #2) — 2026-01-23
- **Repo**: https://github.com/tbcolby/precursor-timers
- **Features**: Pomodoro (25/5/15 min cycles), Stopwatch (centisecond + laps), Countdown collection
- **Key deps**: `gam`, `pddb`, `modals`, `llio`, `ticktimer-server`, `timer-core` (custom lib)
- **Build**: `cargo xtask renode-image timers` — compiles clean (0 warnings)
- **Library crate**: `timer-core` at `libs/timer-core/` — pure Rust, 7 unit tests, host-testable
- **Lessons learned**:
  - `std::thread::spawn` works in Xous for background threads
  - Pump thread pattern: separate SID/CID, `try_receive_message` returns `Result<Option<MessageEnvelope>>`
  - Non-blocking receive + sleep loop for controllable pump intervals
  - Must stop pump on FocusChange::Background, restart on Foreground
  - `AppMode` enum should derive `Copy` to avoid move issues in match arms
  - Borrow checker: extract data from `&mut self.field` before calling other `&mut self` methods
  - PDDB binary serialization: manual `to_le_bytes`/`from_le_bytes` works well for small structs
  - Modals text input: `alert_builder("prompt").field(None, None).build()` → `.first().content`
  - LLIO vibration: `Llio::new(&xns)` then `llio.vibe(llio::VibePattern::Double)`
  - Progress bars: two overlapping rectangles (outline + fill, width = fraction * total)
  - Renode automation: `inject_line("")` is more reliable than `timed_key('Return')` for Enter key in app rawkeys context

### Writer (App #3) — 2026-01-23
- **Repo**: https://github.com/tbcolby/precursor-writer
- **Features**: Markdown Editor (line-level styling, preview, doc management), Journal (date-keyed entries, search), Typewriter (append-only, word count)
- **Key deps**: `gam`, `pddb`, `llio`, `modals`, `writer-core` (custom lib)
- **Build**: `cargo xtask renode-image writer` — compiles clean (0 warnings)
- **Library crate**: `writer-core` at `apps/writer/writer-core/` — pure Rust, 40 unit tests, host-testable
- **Lessons learned**:
  - Esc-prefix key commands: `'\u{001b}'` as leader key avoids conflict with text input
  - Line-level markdown styling: one GlyphStyle per TextView (Large for H1, Bold for H2, Monospace for code)
  - Multi-dictionary PDDB: `writer.docs` and `writer.journal` keep concerns separated
  - Binary index management: `_index` key with count + entries for listing documents/dates
  - Date from `llio::LocalTime`: `get_local_time_ms()` → epoch ms, then manual division for YYYY-MM-DD
  - Viewport scrolling: TextBuffer tracks viewport_top, ensure_cursor_visible adjusts scroll
  - Export via TCP listener (port 7879): same pattern as flashcards import but reversed
  - Standalone repo structure: writer-core nested inside app dir (not in top-level libs/)
