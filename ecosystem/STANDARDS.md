# Precursor App Ecosystem Standards

**32-app campaign — enforced across all builds.**

**Version anchor:** verified against betrusted-io/xous-core **31fbe23** (also in `XOUS_CORE_ANCHOR`).
Every Rust snippet in this file and in `agents/*.md` either compiles there (`tests/doc-snippets/`,
`cargo xtask hosted-ci docsnip`) or is marked `rust,ignore` with a reason.

---

## 1. Keyboard Conventions (Universal)

This is the single key table for the toolkit; other docs link here.

### What apps actually receive (rawkeys)

Source: `services/keyboard/src/mappings/qwerty.rs`, `services/gam/src/contexts.rs:741`,
`emulation/peripherals/keyboard.cs`.

| Key | Char delivered | Renode scancode | Notes |
|-----|----------------|-----------------|-------|
| Arrow Up / Down / Left / Right | `'\u{2191}'` `'\u{2193}'` `'\u{2190}'` `'\u{2192}'` | `Up` `Down` `Left` `Right` | Never Apple PUA U+F700–F703 |
| Enter | `'\r'` (0x0D) | `Enter` (**not** `Return`) | Or `inject_line("")` |
| Backspace | `'\u{0008}'` | `BackSpace` | Use as Back on non-text screens |
| F1 / F2 / F3 / F4 | `'\u{0011}'` … `'\u{0014}'` | `F1` … `F4` | Not U+F001… |
| Space | `' '` | `Space` | |
| Letters, digits, punctuation | the character | `A`–`Z`, `Number0`–`Number9`, `OemComma`, `OemPeriod`, … | Shift: `ShiftL` / `ShiftR`; shift+`.` gives `'😊'` |
| Menu / D-pad centre `∴` (U+2234) | **never delivered** | `Home` | GAM consumes it for app contexts and opens the main menu |
| Escape | **does not exist** | none (no `Escape` in `MatrixMapping`) | Nothing produces 0x1B; the inject path eats 0x1B as an ANSI prefix |

### Standard bindings

| Key | Action | Notes |
|-----|--------|-------|
| `q` | Leave app → `gam.relinquish_focus()` / back from sub-screen | ALWAYS available outside text entry |
| Backspace | Back to previous screen (non-text screens) | Every screen needs an exit reachable with Backspace or F4 |
| Arrows | Navigate items / pages | |
| Enter | Confirm / Select / Submit | |
| Space | Toggle / Secondary action | Context-dependent |
| F1 | App menu / Help | Replaces any "menu key" binding |
| F2 / F3 | App-specific | |
| F4 | Settings / Mode toggle, or Back | |
| `n` `s` `e` `i` `h` | New / Save / Export / Import / Help | Where applicable |

Never bind Menu/Back/Quit to `∴` or ESC: the app will never see either. Command prefixes use an F-key
(`agents/architecture.md` "Command-Prefix Pattern").

### Key Handling Constants

Prefer `precursor_appkit::keys`. Raw equivalent (compiled in `tests/doc-snippets/src/keys.rs`):

```rust
pub const KEY_UP: char = '\u{2191}'; // ↑
pub const KEY_DOWN: char = '\u{2193}'; // ↓
pub const KEY_LEFT: char = '\u{2190}'; // ←
pub const KEY_RIGHT: char = '\u{2192}'; // →
pub const KEY_ENTER: char = '\r'; // 0x0D
pub const KEY_BACKSPACE: char = '\u{0008}';
pub const KEY_SPACE: char = ' ';
pub const KEY_F1: char = '\u{0011}';
pub const KEY_F2: char = '\u{0012}';
pub const KEY_F3: char = '\u{0013}';
pub const KEY_F4: char = '\u{0014}';
// No KEY_MENU: '∴' (U+2234) is consumed by the GAM (opens the main menu) and never reaches apps.
// No KEY_ESC: no Precursor key produces 0x1B.

/// Decode the four scalar args of a Rawkeys message; 0 = no key.
pub fn decode_rawkeys(k1: usize, k2: usize, k3: usize, k4: usize) -> impl Iterator<Item = char> {
    [k1, k2, k3, k4].into_iter().filter_map(|k| char::from_u32(k as u32)).filter(|&c| c != '\u{0}')
}
```

---

## 2. Naming Conventions

### App Names (two different names)
```rust,ignore
// Fragment: assumes xns, gam, sid, AppOp in scope (full version: agents/architecture.md "App Lifecycle")
const SERVER_NAME: &str = "_App Display Name_"; // xous-names server name: any unique string
let sid = xns.register_name(SERVER_NAME, None)?;
// GAM app name: ALWAYS the generated constant. Manifest key "budget-ledger" -> APP_NAME_BUDGET_LEDGER,
// whose value is that entry's context_name. A hand-typed string makes register_ux return Ok(None).
gam.register_ux(gam::UxRegistration { app_name: String::from(gam::APP_NAME_BUDGET_LEDGER), /* ... */ })?;
```
The manifest key must equal the Cargo package name.

### Cargo Package Name
- Lowercase, hyphenated: `qrcode`, `budget-ledger`, `day-planner`
- Short and descriptive

### Repository Name
- Pattern: `precursor-{name}` (e.g., `precursor-qrcode`, `precursor-budget`)

### PDDB Naming
- Dictionary: `{appname}.{category}` (e.g., `qrcode.history`, `budget.accounts`)
- Key: descriptive, lowercase, dot-separated (e.g., `settings`, `item.0001`)
- Dictionary name max: 110 bytes
- Key name max: 94 bytes
- Overwrite = delete in the key's basis, then recreate (no truncate exists); prefer `precursor_appkit::store`

---

## 3. Screen Layout Standard

All apps MUST follow this layout. Size everything from `gam.get_canvas_bounds(gid)`: the content
canvas sits below the status bar, so it is shorter than the 536 px panel. The pixel rows below are
nominal.

```
┌──────────────────────────────────────┐  y=0
│          HEADER (30px)               │  App name + status indicators
├──────────────────────────────────────┤  y=30
│                                      │
│          CONTENT (~460px)            │  Main application content
│                                      │
├──────────────────────────────────────┤  y=490
│          FOOTER (46px)               │  F-key labels, navigation hints
└──────────────────────────────────────┘  y=536
```

- Width: 336px
- Header: Bold/Large font on a light background with a rule underneath (no inverted text: the GAM refuses it for apps), left-aligned app name, right-aligned status
- Footer: F1-F4 labels, evenly spaced (84px per key zone)
- Content: App-specific, use Regular (15px) or Monospace (15px) for text

### UxType
- `UxType::Framebuffer` + `rawkeys_id`: games and tools (the default for this ecosystem).
- `UxType::Chat`: reserves an IME input line at the bottom and delivers typed lines via `gotinput_id`.

---

## 4. Opcode Conventions

```rust
// compiled (as part of a larger enum) in tests/doc-snippets/src/arch.rs
#[derive(Debug, num_derive::FromPrimitive, num_derive::ToPrimitive)]
enum AppOp {
    // GAM callbacks: 0-9
    Redraw = 0,
    Rawkeys = 1,
    FocusChange = 2,

    // Pump / timer: 10-19
    Pump = 10,

    // Internal messages: 20-99
    // (app-specific)

    // User actions: 100+
    // (app-specific)

    // Control: 255
    Quit = 255,
}
```

---

## 5. State Machine Pattern

All apps MUST use an enum-based state machine:

```rust
// compiled in tests/doc-snippets/src/imports.rs
#[derive(Debug, Clone, PartialEq)]
enum AppState {
    // Every app starts here
    MainMenu,
    // App-specific states...
    Help,
    Settings,
}
```

Every app MUST handle:
- `MainMenu` — entry point
- `Help` — accessible via H or F1
- Leave from any state via Q (→ `gam.relinquish_focus()`), and Back via Backspace/F4

---

## 6. Focus Management (Required)

Prefer `precursor_appkit::pump` for the stop/resume part. Match arm (compiled as `focus()` in
`tests/doc-snippets/src/review.rs`):
```rust
Some(AppOp::FocusChange) => xous::msg_scalar_unpack!(msg, state_code, _, _, _, {
    match gam::FocusState::convert_focus_change(state_code) {
        gam::FocusState::Background => {
            allow_redraw = false;
            // Stop pump threads (nothing may tick while another app is in front)
            // Save state if dirty
        }
        gam::FocusState::Foreground => {
            allow_redraw = true;
            // Resume pump threads
            // Trigger redraw
        }
    }
}),
```

---

## 7. PDDB Patterns

### Settings Storage
Every app with settings MUST:
1. Define a `settings` key in its primary dictionary
2. Use serde JSON for settings (not binary) — easier to migrate
3. Load on startup, save on change and on background/quit
4. Handle missing settings gracefully (use defaults)

### Data Collections
- Use an index key listing all item keys
- Each item gets its own PDDB key
- Binary serialization for performance-critical data
- JSON for human-inspectable data

---

## 8. TCP Port Allocation

Reserved ports for the ecosystem:

| Port | App | Direction |
|------|-----|-----------|
| 6464 | C64 Emulator | Import |
| 7878 | Flashcards | Import |
| 7879 | Writer / Flashcards | Export |
| 7880 | QR Code Generator | Import |
| 7881 | Barcode Generator | Import |
| 7882 | Contact Book | Import/Export |
| 7883 | Budget Ledger | Import/Export |
| 7884 | RSS Reader | Import |
| 7885 | JSON Explorer | Import |
| 7886 | Git Log Viewer | Import |
| 7887 | Cipher Workshop | Import/Export |
| 7888 | One-Time Pad | Export |
| 7889 | Key Ceremony | Export |
| 7890 | Math Drill | Import |
| 7891 | Regex Tester | Import |
| 7892 | Conway's Life | Import |
| 7893 | Text Adventure | Import |
| 7894 | Game Boy | Import |
| 7895 | SQLite | Import/Export |
| 7896-7899 | Reserved | Future |

---

## 9. File Structure Standard

Every app repo follows this structure:

```
precursor-{name}/
├── Cargo.toml          # Package config
├── README.md           # Detailed README with philosophy, features, screenshots
├── CLAUDE.md           # Agent instructions for this specific app
├── AGENTS.md           # Specialist agent definitions evolved from this build
├── LICENSE             # Apache 2.0
├── .gitignore          # Standard Rust + Xous ignores
├── screenshots/        # Renode captures, numbered ##_description.png
│   └── 01_initial.png
└── src/
    ├── main.rs         # Entry point, event loop, GAM registration
    ├── app.rs          # App state machine, input dispatch (if >300 LOC)
    ├── ui.rs           # Drawing/rendering (if >300 LOC)
    ├── storage.rs      # PDDB operations (if needed)
    └── ...             # App-specific modules
```

### New additions vs existing apps:
- **CLAUDE.md** — app-specific agent instructions (what was learned building it)
- **AGENTS.md** — specialist agent definitions evolved during this build

---

## 10. README Standard

Every README follows this structure:
1. **Title + tagline** (one-liner philosophy)
2. **Epigraph** (relevant quote in code block)
3. **What This Is** (1-2 paragraphs)
4. **Why This Project** (the philosophical/historical case)
5. **Why Precursor** (how constraints shaped the design)
6. **How It Works** (features, keyboard controls tables)
7. **Screenshots** (numbered, captioned)
8. **Technical Architecture** (file tree, design decisions, PDDB layout)
9. **Building** (integration steps for xous-core)
10. **Development** (link to xous-dev-toolkit)
11. **Author** (Tyler Colby, Colby's Data Movers)
12. **License** (Apache 2.0)

---

## 11. Dependency Versions (Pinned)

Pinned to xous-core **31fbe23**; crate versions are copied from that commit's `apps/hello/Cargo.toml`
(`ticktimer` from `apps/ball/Cargo.toml`). When the anchor moves, re-copy them from there and rerun
`cargo xtask hosted-ci docsnip`.

```toml
# Core (required for every app)
xous = "0.9.71"
xous-ipc = "0.10.11"
log = "0.4.14"
log-server = { package = "xous-api-log", version = "0.1.70" }
xous-names = { package = "xous-api-names", version = "0.9.72" }
num-derive = { version = "0.4.2", default-features = false }
num-traits = { version = "0.2.14", default-features = false }

# Graphics (required for UI apps)
gam = { path = "../../services/gam" }

# Timing (required for pump thread apps)
ticktimer-server = { package = "xous-api-ticktimer", version = "0.9.71" }

# Storage (required for persistent apps)
pddb = { path = "../../services/pddb" }

# Serialization (recommended for settings)
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Random (for TRNG apps)
trng = { path = "../../services/trng" }

# Networking (for TCP apps)
# Use std::net directly — no extra crate needed

# Modals (for dialog apps)
modals = { path = "../../services/modals" }
```

Workspace: add each app to the root `Cargo.toml` **`members`** only, never `default-members`.

### APIs that do not exist (do not use)
`graphics_server::*` in apps · `gam::RawkeysEvent` · `gam.clear()` · `gam.draw_textview()` · `Canvas`
as an app type · `Ticktimer::set_alarm` · `xous::String::from_str` · `PddbKey::set_len` ·
`pddb::PddbError` · `net::Net` (it's `net::NetManager`). Also: `TextView.invert = true` is refused for apps,
and `xous::destroy_server` must be called inside `unsafe { }`.

---

## 12. .gitignore Standard

```
/target/
Cargo.lock
*.swp
*.swo
*~
.DS_Store
```

---

## 13. Build Verification Checklist

Before shipping any app, verify:

- [ ] `cargo xtask hosted-ci {name}` succeeds at the anchor commit (and `cargo xtask renode-image {name}`)
- [ ] Zero compiler warnings
- [ ] `UxRegistration.app_name` is `gam::APP_NAME_{KEY}`; manifest key == package name
- [ ] App is in workspace `members` only
- [ ] One import route (gam re-exports preferred)
- [ ] No do-not-use API (section 11); no `invert`; no `∴`/ESC bindings; TRNG fails closed
- [ ] gam.redraw() called after every draw sequence
- [ ] FocusChange handled (background/foreground)
- [ ] Q leaves (relinquish_focus) from any state
- [ ] PDDB sync() after writes
- [ ] All drawing coordinates use isize
- [ ] Screenshots captured and numbered
- [ ] README complete with all 12 sections
- [ ] CLAUDE.md written with learnings
- [ ] AGENTS.md updated with new specialist agents
- [ ] renode_capture.py updated with capture function
