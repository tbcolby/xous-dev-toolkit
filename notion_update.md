# Notion Project Update — 2026-01-23

## App #2: Timers — SHIPPED

### Status: Complete
- **Repo**: https://github.com/tbcolby/precursor-timers
- **Build**: Clean (0 warnings), PID 27 in Renode image
- **Tests**: 7/7 timer-core unit tests pass
- **Screenshots**: 4 verified (mode_select, pomodoro, stopwatch, settings)

### What Was Built
Unified timer suite with three modes sharing a `timer-core` library crate:

1. **Pomodoro Timer** — 25/5/15 min work/break cycles
   - Auto-transition between phases
   - Progress bar, session counter
   - Configurable via PDDB

2. **Stopwatch** — Centisecond precision (HH:MM:SS.cs)
   - 100ms display refresh via pump thread
   - Up to 99 laps with split times
   - Scrollable lap list

3. **Countdown Collection** — Named timers
   - Create via modals (name + MM:SS duration)
   - Persisted to PDDB
   - Progress bar during countdown

4. **Settings** — Alert configuration
   - Vibration (ON by default)
   - Notification (ON by default)
   - Audio (OFF, requires codec)

### Architecture Highlights
- **timer-core library**: Pure Rust, no Xous deps, host-testable (7 unit tests)
- **Pump thread**: Controllable background thread with adaptive interval (100ms/1000ms)
- **Focus-aware**: Pauses pump when backgrounded, zero CPU waste
- **PDDB persistence**: Binary serialization for settings + countdown list
- **LLIO vibration**: Device vibration alerts on timer events

### Key Metrics
| Metric | Value |
|--------|-------|
| Lines of code | 1,918 |
| Source files | 7 (app) + 1 (library) |
| Dependencies | 11 crates |
| Unit tests | 7 (all pass) |
| Build warnings | 0 |
| Screenshots | 4 |

### New Patterns Established
- Library crate separation (`libs/timer-core/`) for testable logic
- Controllable pump thread (start/stop/quit via scalar messages)
- `try_receive_message()` for non-blocking message checks
- Progress bar rendering (outline rect + fill rect)
- PDDB binary serialization without serde

### Timeline
- Plan + implementation: same session as flashcards completion
- Total: plan → build → test → screenshots → ship in one session

---

## Cumulative Progress

| # | App | Status | Repo |
|---|-----|--------|------|
| 1 | Flashcards | Shipped | [precursor-flashcards](https://github.com/tbcolby/precursor-flashcards) |
| 2 | Timers | Shipped | [precursor-timers](https://github.com/tbcolby/precursor-timers) |
| 3 | TBD | Planned | — |

### Dev Toolkit Updates
- `CLAUDE.md`: Added timers to reference apps, updated pump thread docs, Renode inject_line note
- `discoveries.md`: Added Timers entry with lessons learned
- `renode_capture.py`: Added `capture_timers()` function with inject_line-based Enter

### Backlog / Notes for Next App
- Renode `timed_key('Return')` unreliable for Enter in rawkeys — use `inject_line("")` instead
- Could add countdown_running screenshot in future capture pass
- Audio alerts (codec integration) deferred — marked OFF by default in settings
- Consider apps that exercise: networking, framebuffer graphics, or sensors next
