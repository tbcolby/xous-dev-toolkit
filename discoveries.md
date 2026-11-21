# Precursor Development Discoveries

Accumulated learnings from building Precursor apps. These supplement CLAUDE.md with real-world debugging experiences.

## Screenshot Capture (Renode)

### Common Failures and Solutions

#### 1. PDDB PIN Mismatch
**Symptom**: Screenshots show "Incorrect PIN" dialog instead of app content.

**Cause**: The `renode.bin` flash image has a different PIN than expected, OR the PDDB was never formatted.

**Solution**:
- Use `--init` flag to reset flash and do full PDDB format with known PIN
- The format process takes ~6 minutes in Renode
- After formatting once, the `renode.bin` file retains state - subsequent runs just need unlock

```bash
# Full init (resets flash, formats PDDB with PIN 'a'):
python3 renode_capture.py --init --app myapp

# Quick capture (PDDB already formatted):
python3 renode_capture.py --app myapp
```

#### 2. App Not Appearing in Menu / Switch Fails
**Symptom**: Navigation completes but app doesn't launch; screenshots show Shellchat instead.

**Cause**: App registration pattern doesn't match Xous conventions.

**Solution**: Register with the GAM using the constant generated from `apps/manifest.json`, and give
the xous-names server its own unique name:
```rust,ignore
const SERVER_NAME: &str = "_MyApp_"; // xous-names server name: any unique string

let sid = xns.register_name(SERVER_NAME, None)?;
let token = gam.register_ux(gam::UxRegistration {
    app_name: String::from(gam::APP_NAME_MYAPP), // generated; never retype the string
    // ... (full struct: agents/architecture.md "App Lifecycle")
})?;
```
(Fragment: the full, compiled version is `register()` in `tests/doc-snippets/src/review.rs`.)

**Why this matters**:
- The GAM only accepts names in the generated `EXPECTED_APP_CONTEXTS` (`services/gam/src/apps.rs`),
  and only apps named on the xtask command line get a constant
- `APP_NAME_MYAPP` comes from the manifest key (`"myapp"`, uppercased, `-` -> `_`); its value is the
  manifest `context_name`
- The underscores in `"_MyApp_"` are just a convention; the server name is unrelated to the app name

#### 3. System Reboots During Capture
**Symptom**: Uptime in screenshots jumps backwards (e.g., 0:17:24 → 0:00:30); later screenshots show different state (PIN dialog, Shellchat).

**Cause**: App crash or panic during key handling. Often triggered by:
- Pressing a key the app doesn't guard, triggering unexpected menu actions
- Memory issues during complex operations
- Bugs in state machine transitions

**Solution**:
- Use direct keyboard shortcuts instead of menu navigation where possible
- Prefer direct app-specific keys (e.g., 'N' for New Game, '1' for select) over long menu walks
- Take debug screenshots at each step to identify where failure occurs
- Check for panics in app code triggered by specific key sequences

#### 4. Keys Not Working / Wrong Characters
**Symptom**: Navigation keys (Home, Down, Up) don't work; character keys produce wrong output (a→@).
Also: `timed_key('Return')` / `timed_key('Escape')` do nothing, because those scancodes are not in the
Renode keyboard's `MatrixMapping` (`emulation/peripherals/keyboard.cs`); unmapped keys are dropped.
Use `Enter` (or `inject_line("")`). There is no Escape key at all. Full table: `ecosystem/STANDARDS.md` section 1.

**Cause**: Keyboard hold timing issue. Keys held >500ms emulated time produce hold variants or nothing.

**Solution**: Always use `timed_key()` for reliable key presses:
```python
def timed_key(key, hold_ms=1, after=1.0):
    """Press key with exactly hold_ms emulated time."""
    send('pause')
    send(f'sysbus.keyboard Press {key}')
    send(f'emulation RunFor "0:0:0.{hold_ms:03d}"')
    send(f'sysbus.keyboard Release {key}')
    send('start')
    time.sleep(after)
```

### App-Specific Capture Tips

#### General Pattern
1. Launch app via menu navigation (Home → Down → Home → Down×N → Home)
2. Wait for app initialization (5-10 seconds)
3. Use app-specific direct keys, not Enter for navigation
4. Take screenshots with sufficient delay (3-4 seconds) for screen updates

#### Othello
- Main menu responds to 'N' key (New Game menu directly)
- Difficulty selection responds to '1', '2', '3', '4', '5' keys
- Avoid Enter on main menu (opens F1 menu which can cause issues)

#### Writer
- Mode selection responds to Enter
- **Known bug**: commands use an ESC prefix (Esc+P preview, Esc+Q quit). No Precursor key produces
  0x1B, Renode has no Escape scancode, and the keyboard inject path eats 0x1B as the start of an ANSI
  sequence, so these commands are unreachable on hardware and in Renode (captures named after them
  show the editor with a literal `p`/`q`). Fix: move to an F-key prefix (`agents/architecture.md`
  "Command-Prefix Pattern").

#### Menu key
- `'∴'` (Renode `Home`) is always consumed by the GAM for app contexts (`services/gam/src/contexts.rs:741`):
  it opens the main menu and the app never sees it. Never bind Menu/Back/Quit to it.

#### Flashcards
- Deck list responds to Enter to open deck
- Uses 'I' for import, 'M' for manage menu

## App Registration

### Manifest Entry
```jsonc
{
  "myapp": {                  // must equal the Cargo package name -> gam::APP_NAME_MYAPP
    "context_name": "MyApp",  // becomes the value of APP_NAME_MYAPP (any display string)
    "menu_name": {
      "appmenu.myapp": {
        "en": "My App",
        "en-tts": "My App"
      }
    }
  }
}
```

### Auto-Generated Files
When building with `cargo xtask`, these files are generated:
- `services/gam/src/apps.rs` - Contains `APP_NAME_MYAPP` constant
- `services/status/src/app_autogen.rs` - Contains menu dispatch logic

The `context_name` from manifest becomes the `APP_NAME_*` constant value.

### Debugging Registration Issues
1. Check `apps.rs` contains your app constant
2. Verify the code passes `gam::APP_NAME_MYAPP` (not a hand-typed string)
3. Ensure the app is in `members` (only) in the root Cargo.toml; never `default-members`
4. Rebuild with `cargo xtask renode-image myapp` to regenerate auto files

## Graphics API

### Type Mismatches
Common error: Circle/DrawStyle parameters expect `isize`, not `u16`.

```rust,ignore
// Illustrative wrong/right pair
// Wrong:
Circle::new_with_style(center, radius as u16, style)

// Correct:
let radius: isize = 14;
Circle::new_with_style(center, radius, style)
```

### DrawStyle stroke_width
The `stroke_width` field is `isize`, not `u16` (compiled in `tests/doc-snippets/src/gfx.rs`):
```rust
DrawStyle {
    fill_color: None,
    stroke_color: Some(PixelColor::Dark),
    stroke_width: 3,  // isize, not u16
}
```

## Borrow Checker Patterns

### Mutating Self While Borrowing State
Common pattern when you need to extract data from state, then mutate self:

```rust,ignore
// Illustrative (`..` elides fields); compiled variant: `App::finish` in tests/doc-snippets/src/review.rs
// Wrong - borrow checker error:
if let AppState::Playing { game, mode, .. } = &self.state {
    self.update_stats(*mode);  // Error: can't mutate while borrowing
    self.state = AppState::GameOver { game: game.clone(), .. };
}

// Correct - extract first, then mutate:
let data = if let AppState::Playing { game, mode, .. } = &self.state {
    Some((game.clone(), *mode))
} else {
    None
};
if let Some((game, mode)) = data {
    self.update_stats(mode);
    self.state = AppState::GameOver { game, mode, .. };
}
```

## PDDB

### Timing
- Format: ~6 minutes in Renode
- Mount after unlock: ~45 seconds
- First access to a key after mount: may have slight delay

### Key/Dictionary Naming
- Dictionary names: max 110 bytes (check is `len > 111 - 1`)
- Key names: max 94 bytes (check is `len > 95 - 1`)
- Use dots for namespacing: `myapp.settings`, `myapp.data`

### Overwrites
`PddbKey` has no `set_len`/truncate. Writing a shorter value over a longer one leaves stale trailing
bytes (broken JSON). Rewrite = `delete_key` in the basis reported by `key.attributes()?.basis`, then
recreate (pattern from `apps/vault/src/storage.rs`). Errors are `std::io::Error`; there is no `PddbError`.

## API Drift and Hallucinations

### xous 0.9.71: destroy_server is unsafe
```rust,ignore
// Fragment: `sid` is your server
// safety: server has no pending blocking messages or lent pages
unsafe { xous::destroy_server(sid).unwrap() };
```
This one line was the only build error in calc, c64, timers, writer and othello against xous-core 31fbe23.

### Do-not-use list (agents hallucinated these; none exist for apps)
`graphics_server::*` in apps, `gam::RawkeysEvent`, `gam.clear()`, `gam.draw_textview()`, `Canvas` as an
app type, `Ticktimer::set_alarm`, `xous::String::from_str`, `PddbKey::set_len`, `pddb::PddbError`,
`TextView.invert = true` (exists, but the GAM refuses it for apps).

### Leaving an app
Apps are started once and not respawned. Quit = `gam.relinquish_focus()`; never `xous::terminate_process`.
