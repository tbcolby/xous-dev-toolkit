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

**Solution**: Use separate names for xous-names server and GAM UX registration:
```rust
// CORRECT pattern (matches Writer, Flashcards, Timers):
const SERVER_NAME: &str = "_MyApp_";  // Underscored - for xous names server
const APP_NAME: &str = "MyApp";       // No underscores - for GAM, matches manifest

// In main():
let sid = xns.register_name(SERVER_NAME, None)?;  // Uses _MyApp_
let token = gam.register_ux(gam::UxRegistration {
    app_name: String::from(APP_NAME),  // Uses MyApp
    // ...
})?;
```

**Why this matters**:
- `SERVER_NAME` with underscores prevents name collisions in the xous names server
- `APP_NAME` without underscores must match `context_name` in `manifest.json`
- The GAM uses `APP_NAME` to match against the auto-generated `apps.rs` constants

#### 3. System Reboots During Capture
**Symptom**: Uptime in screenshots jumps backwards (e.g., 0:17:24 → 0:00:30); later screenshots show different state (PIN dialog, Shellchat).

**Cause**: App crash or panic during key handling. Often triggered by:
- Pressing Enter which triggers unexpected menu actions
- Memory issues during complex operations
- Bugs in state machine transitions

**Solution**:
- Use direct keyboard shortcuts instead of menu navigation where possible
- Avoid Enter key for navigation; use app-specific keys (e.g., 'N' for New Game, '1' for select)
- Take debug screenshots at each step to identify where failure occurs
- Check for panics in app code triggered by specific key sequences

#### 4. Keys Not Working / Wrong Characters
**Symptom**: Navigation keys (Home, Down, Up) don't work; character keys produce wrong output (a→@).

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
- Uses Esc-prefix commands (Esc+P for preview, Esc+Q to quit)

#### Flashcards
- Deck list responds to Enter to open deck
- Uses 'I' for import, 'M' for manage menu

## App Registration

### Manifest Entry
```json
{
  "myapp": {
    "context_name": "MyApp",  // Must match APP_NAME in code (no underscores)
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
2. Verify `context_name` in manifest matches `APP_NAME` in code exactly
3. Ensure app is added to both `members` and `default-members` in root Cargo.toml
4. Rebuild with `cargo xtask renode-image myapp` to regenerate auto files

## Graphics API

### Type Mismatches
Common error: Circle/DrawStyle parameters expect `isize`, not `u16`.

```rust
// Wrong:
Circle::new_with_style(center, radius as u16, style)

// Correct:
let radius: isize = 14;
Circle::new_with_style(center, radius, style)
```

### DrawStyle stroke_width
The `stroke_width` field is `isize`, not `u16`:
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

```rust
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
- Dictionary names: max 111 characters
- Key names: max 95 characters
- Use dots for namespacing: `myapp.settings`, `myapp.data`
