# Precursor App Ecosystem Standards

**32-app campaign — enforced across all builds.**

---

## 1. Keyboard Conventions (Universal)

Every app MUST use these standard key mappings. No exceptions.

| Key | Unicode | Action | Notes |
|-----|---------|--------|-------|
| Q | `q` | Quit app / Back to previous screen | ALWAYS available from any screen |
| Arrow Up | `\u{2191}` | Navigate up / Previous item | |
| Arrow Down | `\u{2193}` | Navigate down / Next item | |
| Arrow Left | `\u{2190}` | Navigate left / Previous page | |
| Arrow Right | `\u{2192}` | Navigate right / Next page | |
| Enter | `\r` | Confirm / Select / Submit | |
| Space | `\u{0020}` | Toggle / Secondary action | Context-dependent |
| Backspace | `\u{0008}` | Delete / Clear entry | |
| Home/Menu | `\u{2234}` (∴) | Open context menu | If app has menus |
| F1 | Function key | Help / Primary function menu | App-specific |
| F2 | Function key | Secondary function | App-specific |
| F3 | Function key | Tertiary function | App-specific |
| F4 | Function key | Settings / Mode toggle | App-specific |
| N | `n` | New item / New game | Where applicable |
| S | `s` | Save / Store | Where applicable |
| E | `e` | Export | Where applicable |
| I | `i` | Import | Where applicable |
| H | `h` | Help screen | Where applicable |

### Key Handling Constants

All apps MUST define these constants for consistency:

```rust
// Standard key codes
const KEY_UP: char = '\u{2191}';
const KEY_DOWN: char = '\u{2193}';
const KEY_LEFT: char = '\u{2190}';
const KEY_RIGHT: char = '\u{2192}';
const KEY_ENTER: char = '\r';
const KEY_BACKSPACE: char = '\u{0008}';
const KEY_MENU: char = '\u{2234}';
const KEY_SPACE: char = ' ';
```

---

## 2. Naming Conventions

### Rust Constants
```rust
const SERVER_NAME: &str = "_App Display Name_";  // Underscored, for xous names server
const APP_NAME: &str = "App Display Name";        // No underscores, matches manifest context_name
```

### Cargo Package Name
- Lowercase, hyphenated: `qrcode`, `budget-ledger`, `day-planner`
- Short and descriptive

### Repository Name
- Pattern: `precursor-{name}` (e.g., `precursor-qrcode`, `precursor-budget`)

### PDDB Naming
- Dictionary: `{appname}.{category}` (e.g., `qrcode.history`, `budget.accounts`)
- Key: descriptive, lowercase, dot-separated (e.g., `settings`, `item.0001`)
- Dictionary name max: 111 characters
- Key name max: 95 characters

---

## 3. Screen Layout Standard

All apps using `UxType::Chat` MUST follow this layout:

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
- Header: Bold/Large font, left-aligned app name, right-aligned status
- Footer: F1-F4 labels, evenly spaced (84px per key zone)
- Content: App-specific, use Regular (15px) or Monospace (15px) for text

### Framebuffer Apps
Apps using `UxType::Framebuffer` have full 336x536 pixel control. Reserve bottom 30-40px for status bar.

---

## 4. Opcode Conventions

```rust
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
- Graceful quit from any state via Q

---

## 6. Focus Management (Required)

```rust
Some(AppOp::FocusChange) => xous::msg_scalar_unpack!(msg, state_code, _, _, _, {
    match gam::FocusState::convert_focus_change(state_code) {
        gam::FocusState::Background => {
            allow_redraw = false;
            // Stop pump threads
            // Save state
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

All apps use these exact versions:

```toml
# Core (required for every app)
xous = "0.9.69"
xous-ipc = "0.10.9"
log = "0.4.14"
log-server = { package = "xous-api-log", version = "0.1.68" }
xous-names = { package = "xous-api-names", version = "0.9.70" }
num-derive = { version = "0.4.2", default-features = false }
num-traits = { version = "0.2.14", default-features = false }

# Graphics (required for UI apps)
gam = { path = "../../services/gam" }

# Timing (required for pump thread apps)
ticktimer-server = { package = "xous-api-ticktimer", version = "0.9.68" }

# Storage (required for persistent apps)
pddb = { path = "../../services/pddb" }

# Serialization (recommended for settings)
serde = { version = "1.0", default-features = false, features = ["derive", "alloc"] }
serde_json = { version = "1.0", default-features = false, features = ["alloc"] }

# Random (for TRNG apps)
trng = { path = "../../services/trng" }

# Networking (for TCP apps)
# Use std::net directly — no extra crate needed

# Modals (for dialog apps)
modals = { path = "../../services/modals" }
```

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

- [ ] `cargo build -p {name} --target riscv32imac-unknown-xous-elf` succeeds
- [ ] Zero compiler warnings
- [ ] APP_NAME matches manifest.json context_name exactly
- [ ] SERVER_NAME has underscores, APP_NAME does not
- [ ] No direct ux-api or blitstr2 imports
- [ ] gam.redraw() called after every draw sequence
- [ ] FocusChange handled (background/foreground)
- [ ] Q quits from any state
- [ ] PDDB sync() after writes
- [ ] All drawing coordinates use isize
- [ ] Screenshots captured and numbered
- [ ] README complete with all 12 sections
- [ ] CLAUDE.md written with learnings
- [ ] AGENTS.md updated with new specialist agents
- [ ] renode_capture.py updated with capture function
