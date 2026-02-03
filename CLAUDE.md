# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository Purpose

This is the **Xous Development Toolkit** — automation tools and specialized agents for developing Precursor/Xous apps. It provides:

- **Multi-agent system** for full development lifecycle
- **Python automation** for headless Renode testing on macOS ARM64
- **Domain specialists** for ideation, architecture, graphics, storage, networking, build, testing, and review

## Agent System

This toolkit uses a **supervisor/specialist agent model** for comprehensive app development. See `agents/README.md` for full documentation.

### Quick Reference

| Agent | Invoke When... |
|-------|----------------|
| **Supervisor** | Starting a new task, need routing |
| **Ideation** | Conceiving apps, designing UX, prioritizing features |
| **Architecture** | Designing state machines, opcodes, module structure |
| **Graphics** | Implementing UI, text rendering, drawing |
| **Storage** | PDDB persistence, serialization, key management |
| **Networking** | TCP/UDP, HTTP, TLS, background sync |
| **Build** | Cargo.toml, manifest.json, workspace, deployment |
| **Testing** | Renode automation, screenshots, validation |
| **Review** | Code review, patterns, performance, pitfalls |

### Development Flow
```
User Request → Supervisor → Ideation → Architecture
                              ↓
                    Graphics / Storage / Networking
                              ↓
                    Build → Testing → Review → Ship
```

Load an agent's context by reading `agents/<name>.md` when working in that domain.

---

## Python Automation

The toolkit includes Python scripts for headless Renode testing:

## Repository Structure

```
xous-dev-toolkit/
├── agents/                  # Specialist agent definitions
│   ├── README.md            # Agent system overview
│   ├── supervisor.md        # Task routing and orchestration
│   ├── ideation.md          # App concepts and UX design
│   ├── architecture.md      # Code structure and patterns
│   ├── graphics.md          # GAM API and UI implementation
│   ├── storage.md           # PDDB persistence patterns
│   ├── networking.md        # TCP/UDP, HTTP, TLS
│   ├── build.md             # Cargo, manifest, deployment
│   ├── testing.md           # Renode automation and QA
│   └── review.md            # Code review and quality
├── scripts/
│   ├── renode_capture.py    # Full automation: boot, PDDB init, app launch, screenshots
│   ├── renode_interact.py   # Low-level Renode control (legacy, simpler API)
│   └── usb_log_monitor.py   # USB serial log viewer for hardware debugging
├── CLAUDE.md                # This file (toolkit-specific guidance)
├── README.md                # User documentation
└── discoveries.md           # Debugging notes and API discoveries
```

## Common Commands

### Screenshot Capture (Primary Use Case)

```bash
cd scripts

# Full PDDB init (blank flash) + app capture:
python3 renode_capture.py --init --app flashcards --screenshots ../output

# Quick capture (PDDB already formatted with PIN 'a'):
python3 renode_capture.py --app flashcards --screenshots ../output

# Custom app index (if app isn't first in submenu):
python3 renode_capture.py --app othello --app-index 2 --screenshots ../output
```

### USB Log Monitoring (Hardware)

```bash
# First, on the Precursor: open shellchat, type "usb console"
# Then on your Mac:
python3 scripts/usb_log_monitor.py                    # Auto-detect
python3 scripts/usb_log_monitor.py --filter myapp     # Filter by keyword
python3 scripts/usb_log_monitor.py --save debug.log   # Save to file
```

### Low-Level Renode Interaction

```bash
# Take a screenshot (requires Renode already running on port 4567)
python3 scripts/renode_interact.py screenshot output.png

# Press a key
python3 scripts/renode_interact.py press-key Home

# Full init sequence
python3 scripts/renode_interact.py full-init 90
```

## Architecture

### RenodeController Class (`renode_capture.py`)

The core automation class that communicates with Renode via telnet:

```python
class RenodeController:
    def timed_key(key, hold_ms=1, after=1.0)  # Safe key press with timing control
    def inject_line(text)                      # Direct text injection (bypasses timing)
    def screenshot(filepath)                   # Capture LCD as PNG
    def init_pddb(pin)                         # Full PDDB format sequence
    def unlock_pddb(pin)                       # Unlock already-formatted PDDB
    def launch_app(app_index)                  # Navigate menu to launch app
```

### Key Timing Problem & Solutions

The Xous keyboard has a **500ms hold threshold**. Keys held longer produce hold variants or are dropped entirely. Two solutions:

1. **`timed_key()`** — Pauses emulation, presses key, advances exactly 1ms, releases. Use for navigation keys.
2. **`inject_line()`** — Injects text directly into keyboard buffer. Use for PIN entry and text input.

### Adding Support for New Apps

1. Create a `capture_<appname>()` function in `renode_capture.py`:

```python
def capture_myapp(ctl, screenshot_dir):
    """Capture all myapp screenshots."""
    def ss(name):
        return ctl.screenshot(os.path.join(screenshot_dir, name))

    def enter(after=3.0):
        ctl.inject_line("")
        time.sleep(after)

    print("\n=== MyApp Screenshots ===", flush=True)

    # Initial state
    ss("initial.png")

    # Navigate and capture
    ctl.timed_key('Down', after=2.0)
    enter()
    ss("next_screen.png")

    print("=== Done! ===", flush=True)
```

2. Add the app to the dispatch in `main()`:

```python
elif args.app == 'myapp':
    capture_myapp(ctl, screenshot_dir)
```

## Configuration

Key constants at the top of `renode_capture.py`:

```python
MONITOR_PORT = 4567          # Renode telnet port
XOUS_ROOT = "/path/to/xous-core"  # Update for your system
RENODE = "/Applications/Renode.app/Contents/MacOS/renode"
DEFAULT_PIN = "a"            # PDDB PIN for test images
```

## Dependencies

- Python 3.x (standard library only for renode scripts)
- `pyserial` (only for usb_log_monitor.py): `pip install pyserial`
- Renode emulator with ARM64 support

## Key Scan Codes Reference

| Key | Code | Notes |
|-----|------|-------|
| Letters | `A`-`Z` | Uppercase scan codes |
| Numbers | `Number0`-`Number9` | |
| Navigation | `Up`, `Down`, `Left`, `Right` | Timing-critical (use `timed_key`) |
| Menu/Select | `Home` | Opens menu AND selects items |
| Enter | `Return` | |
| Function | `F1`, `F2`, `F3`, `F4` | |
| Shift | `ShiftL`, `ShiftR` | NOT `ShiftLeft`/`ShiftRight` |

## Debugging Tips

- **Keys not working**: Use `timed_key()` instead of bare press/release
- **Wrong characters**: Key held too long; reduce `after` parameter or use `inject_line()`
- **App not launching**: Check app index in menu; Shellchat is index 0
- **PIN mismatch**: Use `--init` flag to reset and reformat PDDB
- **Screenshot shows wrong state**: Increase `time.sleep()` delays for slower operations
