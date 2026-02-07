#!/usr/bin/env python3
"""
Renode automation for Precursor/Xous apps.

Handles PDDB initialization, app navigation, and screenshot capture.
Uses renode_lib.py for core Renode control (timed_key, inject_line, screenshot).

Usage:
    # Full init (blank flash) + capture flashcards screenshots:
    python3 renode_capture.py --init --app flashcards

    # Quick capture (PDDB already formatted with PIN 'a'):
    python3 renode_capture.py --app flashcards

    # Just init PDDB (no app launch):
    python3 renode_capture.py --init

    # Custom app with custom capture script:
    python3 renode_capture.py --app myapp --script my_captures.py

Environment variables (see renode_lib.py):
    RENODE_PATH     Path to Renode binary
    XOUS_ROOT       Path to xous-core checkout
    RENODE_PORT     Monitor telnet port (default: 4567)
"""

import time
import sys
import os
import argparse

# Add scripts dir to path for import
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from renode_lib import RenodeController, start_renode, reset_flash, XOUS_ROOT, DEFAULT_PIN


def capture_timers(ctl, screenshot_dir):
    """Capture all timers app screenshots."""
    def ss(name):
        return ctl.screenshot(os.path.join(screenshot_dir, name))

    def enter(after=3.0):
        """Send Enter key via inject_line (CR), which reliably produces '\\r'."""
        ctl.inject_line("")
        time.sleep(after)

    print("\n=== Timers Screenshots ===", flush=True)

    # 1. Mode select screen (initial state, cursor on Pomodoro)
    print("  mode_select...", flush=True)
    time.sleep(2)
    ss("mode_select.png")

    # 2. Pomodoro (cursor already on Pomodoro, Enter to select)
    print("  pomodoro...", flush=True)
    enter()  # Enter Pomodoro mode
    enter()  # Start timer
    time.sleep(3)
    ss("pomodoro.png")

    # 3. Settings (press 's' from Pomodoro)
    print("  settings...", flush=True)
    ctl.timed_key('S', after=3.0)
    ss("settings.png")
    ctl.timed_key('Q', after=3.0)  # Back to Pomodoro
    ctl.timed_key('Q', after=3.0)  # Back to mode select

    # 4. Stopwatch (Down once to Stopwatch, Enter to select)
    print("  stopwatch...", flush=True)
    ctl.timed_key('Down', after=2.0)
    enter()  # Enter Stopwatch mode
    enter()  # Start stopwatch
    time.sleep(3)
    ctl.timed_key('L', after=2.0)  # Lap 1
    time.sleep(2)
    ctl.timed_key('L', after=2.0)  # Lap 2
    time.sleep(1)
    ss("stopwatch.png")
    enter()  # Pause
    ctl.timed_key('Q', after=3.0)  # Back to mode select

    # 5. Countdown list (Down twice to Countdown, Enter)
    print("  countdown_list...", flush=True)
    ctl.timed_key('Down', after=1.0)
    ctl.timed_key('Down', after=1.0)
    enter()  # Enter Countdown mode
    ss("countdown_list.png")
    ctl.timed_key('Q', after=3.0)  # Back to mode select

    print("=== Done! ===", flush=True)


def capture_flashcards(ctl, screenshot_dir):
    """Capture all flashcards app screenshots."""
    def ss(name):
        return ctl.screenshot(os.path.join(screenshot_dir, name))

    print("\n=== Flashcards Screenshots ===", flush=True)

    ss("deck_list.png")

    print("  deck_menu...", flush=True)
    ctl.timed_key('M', after=3.0)
    ss("deck_menu.png")
    ctl.timed_key('Q', after=2.0)

    print("  question/answer...", flush=True)
    ctl.inject_line("")  # CR opens deck for review
    time.sleep(3)
    ss("question.png")
    ctl.timed_key('Space', after=2.0)
    ss("answer.png")
    ctl.timed_key('Q', after=2.0)

    print("  import_wait...", flush=True)
    ctl.timed_key('I', after=3.0)
    ss("import_wait.png")

    print("=== Done! ===", flush=True)


def capture_c64(ctl, screenshot_dir):
    """Capture all C64 emulator app screenshots."""
    def ss(name):
        return ctl.screenshot(os.path.join(screenshot_dir, name))

    def enter(after=3.0):
        ctl.inject_line("")
        time.sleep(after)

    print("\n=== C64 Emulator Screenshots ===", flush=True)

    # 1. Menu screen (initial state, cursor on first item or "Import Game")
    print("  menu...", flush=True)
    time.sleep(2)
    ss("menu.png")

    # 2. Navigate to "Boot C64 (no game)" - it's at the bottom of the menu
    # Move cursor down to "Boot C64 (no game)" (skip "Import Game")
    print("  boot_c64...", flush=True)
    ctl.timed_key('Down', after=1.0)  # to "Import Game"
    ctl.timed_key('Down', after=1.0)  # to "Boot C64"
    ss("menu_boot.png")

    # 3. Boot the emulator (Enter on "Boot C64")
    print("  running emulator...", flush=True)
    enter()  # Select "Boot C64"
    time.sleep(5)  # Wait for boot and initial render
    ss("running.png")

    # 4. Back to menu via Menu key (map_key uses '\u{2234}' but we need Renode key)
    # The Menu key on Precursor sends character code - let's use Home which often maps to menu
    print("  back to menu...", flush=True)
    ctl.timed_key('Home', after=3.0)
    ss("menu_return.png")

    # 5. Import screen
    print("  import_wait...", flush=True)
    ctl.timed_key('Up', after=1.0)  # Back to "Import Game"
    enter()  # Select Import
    time.sleep(3)
    ss("import_wait.png")

    # Note: Import will timeout eventually, returning to menu
    print("=== Done! ===", flush=True)


def capture_writer(ctl, screenshot_dir):
    """Capture all writer app screenshots."""
    def ss(name):
        return ctl.screenshot(os.path.join(screenshot_dir, name))

    def enter(after=3.0):
        ctl.inject_line("")
        time.sleep(after)

    def esc_cmd(key_name, after=3.0):
        """Send Esc followed by a command key."""
        ctl.timed_key('Escape', after=0.5)
        ctl.timed_key(key_name, after=after)

    print("\n=== Writer Screenshots ===", flush=True)

    # 1. Mode select screen (initial state)
    print("  mode_select...", flush=True)
    time.sleep(2)
    ss("mode_select.png")

    # 2. Enter Markdown Editor (cursor already on it, press Enter)
    print("  editor...", flush=True)
    enter()  # → DocList
    time.sleep(2)

    # 3. New document (press 'n')
    ctl.timed_key('N', after=3.0)
    time.sleep(2)

    # 4. Type some markdown content
    ctl.inject_line("# My Document")
    time.sleep(1)
    ctl.inject_line("")  # blank line
    time.sleep(0.5)
    ctl.inject_line("This is a **writer** app for Precursor.")
    time.sleep(0.5)
    ctl.inject_line("")
    time.sleep(0.5)
    ctl.inject_line("## Features")
    time.sleep(0.5)
    ctl.inject_line("")
    time.sleep(0.5)
    ctl.inject_line("- Markdown styling")
    time.sleep(0.5)
    ctl.inject_line("- Journal mode")
    time.sleep(0.5)
    ctl.inject_line("- Typewriter mode")
    time.sleep(0.5)
    ctl.inject_line("")
    time.sleep(0.5)
    ctl.inject_line("> Built for focused writing")
    time.sleep(0.5)
    ctl.inject_line("")
    time.sleep(0.5)
    ctl.inject_line("---")
    time.sleep(2)

    # 5. Screenshot editor in edit mode
    ss("editor_edit.png")

    # 6. Preview mode (Esc + p)
    print("  editor_preview...", flush=True)
    esc_cmd('P', after=3.0)
    ss("editor_preview.png")

    # 7. Back to edit, then export menu (Esc + e)
    print("  export_menu...", flush=True)
    esc_cmd('P', after=2.0)  # toggle back to edit
    esc_cmd('E', after=3.0)  # export menu
    ss("export_menu.png")

    # 8. Cancel export, back to doc list, back to mode select
    ctl.timed_key('Q', after=2.0)  # cancel export
    esc_cmd('Q', after=3.0)  # back to doc list
    ctl.timed_key('Q', after=3.0)  # back to mode select

    # 9. Journal mode (Down once, Enter)
    print("  journal...", flush=True)
    ctl.timed_key('Down', after=2.0)
    enter()  # Enter journal
    time.sleep(3)

    # Type some journal content
    ctl.inject_line("Today I started using the Writer app.")
    time.sleep(0.5)
    ctl.inject_line("It has markdown support and auto-saves.")
    time.sleep(0.5)
    ctl.inject_line("")
    time.sleep(0.5)
    ctl.inject_line("Planning to write more tomorrow.")
    time.sleep(2)
    ss("journal.png")

    # Back to mode select
    esc_cmd('Q', after=3.0)

    # 10. Typewriter mode (Down twice from top, Enter)
    print("  typewriter...", flush=True)
    ctl.timed_key('Down', after=1.0)
    ctl.timed_key('Down', after=1.0)
    enter()  # Enter typewriter
    time.sleep(2)

    # Type some freewriting content
    ctl.inject_line("The quick brown fox jumps over the lazy dog.")
    time.sleep(0.5)
    ctl.inject_line("Writing flows freely when you cannot edit.")
    time.sleep(0.5)
    ctl.inject_line("Just keep moving forward.")
    time.sleep(2)
    ss("typewriter.png")

    print("=== Done! ===", flush=True)


def capture_calc(ctl, screenshot_dir):
    """Capture scientific calculator screenshots showing algebraic and RPN modes."""
    def ss(name):
        return ctl.screenshot(os.path.join(screenshot_dir, name))

    def enter(after=2.0):
        ctl.inject_line("")
        time.sleep(after)

    def inject_key(char):
        """Inject a single character via keyboard."""
        ctl._send(f'sysbus.keyboard InjectKey {ord(char)}')
        time.sleep(0.3)

    def inject_str(text):
        """Inject a string."""
        ctl._send(f'sysbus.keyboard InjectString "{text}"')
        time.sleep(0.5)

    print("\n=== Calculator Screenshots ===", flush=True)

    # 1. Initial state (algebraic mode)
    print("  [1] Initial state...", flush=True)
    time.sleep(3)
    ss("01_initial.png")

    # 2. Enter an expression: 2+3*4
    print("  [2] Expression: 2+3*4...", flush=True)
    inject_str("2+3*4")
    time.sleep(1)
    ss("02_expression.png")

    # 3. Evaluate (press Enter)
    print("  [3] Evaluate result = 14...", flush=True)
    enter(after=2.0)
    ss("03_result.png")

    # 4. Try sin(90)
    print("  [4] sin(90)...", flush=True)
    inject_str("sin(90)")
    time.sleep(1)
    ss("04_sin90_input.png")

    enter(after=2.0)
    ss("05_sin90_result.png")

    # 5. Try sqrt(2)
    print("  [5] sqrt(2)...", flush=True)
    inject_str("sqrt(2)")
    time.sleep(1)
    enter(after=2.0)
    ss("06_sqrt2.png")

    # 6. Try parentheses: (2+3)*4
    print("  [6] (2+3)*4...", flush=True)
    inject_str("(2+3)*4")
    time.sleep(1)
    ss("07_parens_input.png")

    enter(after=2.0)
    ss("08_parens_result.png")

    # 7. Toggle to RPN mode (press 'M' uppercase)
    print("  [7] Switch to RPN mode...", flush=True)
    inject_key('M')
    time.sleep(2)
    ss("09_rpn_mode.png")

    # 8. RPN: 2 Enter
    print("  [8] RPN: 2 Enter...", flush=True)
    inject_key('2')
    time.sleep(0.5)
    ss("10_rpn_digit2.png")

    enter(after=1.5)
    ss("11_rpn_enter.png")

    # 9. RPN: 3 +
    print("  [9] RPN: 3 +...", flush=True)
    inject_key('3')
    time.sleep(0.5)
    ss("12_rpn_digit3.png")

    inject_key('+')
    time.sleep(1)
    ss("13_rpn_add.png")

    # 10. Clear and do more complex: 2 Enter 3 * 4 +
    print("  [10] RPN: 2 3 * 4 + = 10...", flush=True)
    inject_key('C')  # Clear
    time.sleep(0.5)
    inject_key('2')
    time.sleep(0.3)
    enter(after=0.5)
    inject_key('3')
    time.sleep(0.3)
    inject_key('*')
    time.sleep(0.5)
    inject_key('4')
    time.sleep(0.3)
    inject_key('+')
    time.sleep(1)
    ss("14_rpn_complex.png")

    # 11. Clear and try RPN sqrt
    print("  [11] RPN: sqrt...", flush=True)
    inject_key('C')
    time.sleep(0.5)
    inject_key('2')
    time.sleep(0.3)
    # sqrt is shift+6 - use timed key with shift
    ctl._send('pause')
    time.sleep(0.2)
    ctl._send('sysbus.keyboard Press ShiftL')
    time.sleep(0.1)
    ctl._send('emulation RunFor "0:0:0.001"')
    time.sleep(0.1)
    ctl._send('sysbus.keyboard Press Number6')
    time.sleep(0.1)
    ctl._send('emulation RunFor "0:0:0.001"')
    time.sleep(0.1)
    ctl._send('sysbus.keyboard Release Number6')
    ctl._send('sysbus.keyboard Release ShiftL')
    ctl._send('start')
    time.sleep(1)
    ss("15_rpn_sqrt.png")

    # 12. Toggle angle mode (press 'a')
    print("  [12] Toggle angle mode to RAD...", flush=True)
    inject_key('A')
    time.sleep(1)
    ss("16_rad_mode.png")

    # 13. Toggle back to ALG mode
    print("  [13] Back to ALG mode...", flush=True)
    inject_key('M')
    time.sleep(1)
    ss("17_alg_rad.png")

    # 14. Store to memory (s + 0)
    print("  [14] Memory store...", flush=True)
    inject_str("42")
    time.sleep(0.5)
    enter(after=1.0)
    inject_key('s')
    time.sleep(0.5)
    inject_key('0')
    time.sleep(1)
    ss("18_memory_store.png")

    # 15. Toggle to HEX base (press 'b')
    print("  [15] HEX mode...", flush=True)
    inject_str("255")
    time.sleep(0.5)
    enter(after=1.0)
    inject_key('b')
    time.sleep(1)
    ss("19_hex_mode.png")

    print("=== Done! 19 screenshots captured ===", flush=True)


def capture_carse(ctl, screenshot_dir):
    """Capture Carse Automata screenshots: grid, cursor, touch, simulation, death, reseed."""
    def ss(name):
        return ctl.screenshot(os.path.join(screenshot_dir, name))

    print("\n=== Carse Automata Screenshots ===", flush=True)

    # 1. Initial grid (freshly seeded)
    print("  [1] Initial grid...", flush=True)
    time.sleep(3)
    ss("01_initial.png")

    # 2. Cursor movement — move right and down
    print("  [2] Cursor movement...", flush=True)
    for _ in range(5):
        ctl.timed_key('Right', after=0.3)
    for _ in range(3):
        ctl.timed_key('Down', after=0.3)
    time.sleep(1)
    ss("02_cursor_moved.png")

    # 3. Touch cells — tap space on several cells
    print("  [3] Touching cells...", flush=True)
    ctl.timed_key('Space', after=1.0)
    ctl.timed_key('Right', after=0.3)
    ctl.timed_key('Space', after=1.0)
    ctl.timed_key('Right', after=0.3)
    ctl.timed_key('Space', after=1.0)
    ctl.timed_key('Down', after=0.3)
    ctl.timed_key('Space', after=1.0)
    time.sleep(1)
    ss("03_touched_cells.png")

    # 4. Run simulation — hold space (rapid presses simulate hold)
    print("  [4] Running simulation...", flush=True)
    # Rapid space presses within 700ms window triggers simulation_running
    for _ in range(15):
        ctl.timed_key('Space', after=0.3)
    time.sleep(2)
    ss("04_simulation_running.png")

    # 5. Keep running to see evolution
    print("  [5] Simulation evolution...", flush=True)
    for _ in range(20):
        ctl.timed_key('Space', after=0.3)
    time.sleep(2)
    ss("05_simulation_evolved.png")

    # 6. Let it continue running further
    print("  [6] More evolution...", flush=True)
    for _ in range(30):
        ctl.timed_key('Space', after=0.3)
    time.sleep(3)
    ss("06_simulation_later.png")

    # 7. Pause (stop pressing space, wait for 700ms timeout)
    print("  [7] Paused state...", flush=True)
    time.sleep(3)
    ss("07_paused.png")

    # 8. Resume and run until death (if it happens)
    print("  [8] Running toward death...", flush=True)
    for _ in range(50):
        ctl.timed_key('Space', after=0.3)
    time.sleep(3)
    ss("08_toward_death.png")

    # 9. Continue even more
    print("  [9] Extended run...", flush=True)
    for _ in range(50):
        ctl.timed_key('Space', after=0.3)
    time.sleep(3)
    ss("09_extended.png")

    # 10. Final state
    print("  [10] Final state...", flush=True)
    time.sleep(2)
    ss("10_final.png")

    print("=== Done! 10 screenshots captured ===", flush=True)


def capture_othello(ctl, screenshot_dir):
    """Capture comprehensive Othello screenshots including full games at each AI level."""
    def ss(name):
        return ctl.screenshot(os.path.join(screenshot_dir, name))

    def enter(after=3.0):
        ctl.inject_line("")
        time.sleep(after)

    def play_game(ai_wait=2, max_moves=70):
        """Play moves until game ends or max_moves reached."""
        move_count = 0
        # Scan board making moves - cursor wraps around
        for _ in range(max_moves):
            # Try different directions to find valid moves
            ctl.timed_key('Right', after=0.3)
            enter(after=ai_wait)
            move_count += 1
            if move_count % 8 == 0:
                ctl.timed_key('Down', after=0.3)
            if move_count % 20 == 0:
                print(f"      ...move {move_count}", flush=True)

    print("\n=== Othello Comprehensive Screenshots ===", flush=True)

    # ========== MAIN MENU ==========
    print("  [1] Main menu...", flush=True)
    time.sleep(3)
    ss("01_main_menu.png")

    # ========== NEW GAME MENU ==========
    print("  [2] New game menu...", flush=True)
    ctl.timed_key('N', after=3.0)
    ss("02_new_game_menu.png")

    # ========== EASY GAME ==========
    print("  [3] Starting Easy game...", flush=True)
    ctl.timed_key('Number1', after=5.0)
    time.sleep(3)
    ss("03_easy_start.png")

    print("  [4] Playing Easy game (this takes a few minutes)...", flush=True)
    play_game(ai_wait=2, max_moves=65)
    time.sleep(3)
    ss("04_easy_end.png")

    # Back to main menu
    print("  [5] Returning to menu...", flush=True)
    ctl.timed_key('F4', after=3.0)
    enter(after=2.0)  # Dismiss any dialog
    time.sleep(2)

    # ========== MEDIUM GAME ==========
    print("  [6] Starting Medium game...", flush=True)
    ctl.timed_key('N', after=3.0)
    ctl.timed_key('Number2', after=5.0)
    time.sleep(3)
    ss("05_medium_start.png")

    print("  [7] Playing Medium game...", flush=True)
    play_game(ai_wait=3, max_moves=65)
    time.sleep(3)
    ss("06_medium_end.png")

    ctl.timed_key('F4', after=3.0)
    enter(after=2.0)
    time.sleep(2)

    # ========== HARD GAME ==========
    print("  [8] Starting Hard game...", flush=True)
    ctl.timed_key('N', after=3.0)
    ctl.timed_key('Number3', after=5.0)
    time.sleep(3)
    ss("07_hard_start.png")

    print("  [9] Playing Hard game...", flush=True)
    play_game(ai_wait=4, max_moves=65)
    time.sleep(3)
    ss("08_hard_end.png")

    ctl.timed_key('F4', after=3.0)
    enter(after=2.0)
    time.sleep(2)

    # ========== EXPERT GAME ==========
    print("  [10] Starting Expert game...", flush=True)
    ctl.timed_key('N', after=3.0)
    ctl.timed_key('Number4', after=5.0)
    time.sleep(3)
    ss("09_expert_start.png")

    print("  [11] Playing Expert game...", flush=True)
    play_game(ai_wait=5, max_moves=65)
    time.sleep(3)
    ss("10_expert_end.png")

    ctl.timed_key('F4', after=3.0)
    enter(after=2.0)
    time.sleep(2)

    # ========== TWO PLAYER MODE ==========
    print("  [12] Two Player mode...", flush=True)
    ctl.timed_key('N', after=3.0)
    ctl.timed_key('Number5', after=5.0)
    time.sleep(3)
    ss("11_two_player.png")

    # Make a few moves to show two-player gameplay
    for _ in range(6):
        ctl.timed_key('Right', after=0.5)
        enter(after=1.5)

    ss("12_two_player_midgame.png")
    ctl.timed_key('F4', after=3.0)
    time.sleep(2)

    # ========== IN-GAME F1 MENU ==========
    print("  [13] In-game F1 menu...", flush=True)
    ctl.timed_key('N', after=3.0)
    ctl.timed_key('Number1', after=5.0)
    time.sleep(3)
    # Make a few moves first
    for _ in range(4):
        ctl.timed_key('Right', after=0.5)
        enter(after=2.0)
    ctl.timed_key('F1', after=3.0)
    ss("13_f1_menu.png")
    ctl.timed_key('F4', after=2.0)

    # ========== HINT FEATURE ==========
    print("  [14] Hint feature...", flush=True)
    ctl.timed_key('F1', after=2.0)
    # Navigate: Help(0), MoveHistory(1), Hint(2)
    ctl.timed_key('Down', after=0.5)
    ctl.timed_key('Down', after=0.5)
    enter(after=3.0)
    ss("14_hint.png")

    # ========== SETTINGS ==========
    print("  [15] Settings...", flush=True)
    ctl.timed_key('F4', after=2.0)  # Exit current game
    time.sleep(2)
    ctl.timed_key('S', after=3.0)
    ss("15_settings.png")

    # Toggle a setting
    enter(after=2.0)
    ss("16_settings_toggled.png")
    ctl.timed_key('F4', after=2.0)

    # ========== STATISTICS ==========
    print("  [16] Statistics (should show game results)...", flush=True)
    ctl.timed_key('F1', after=2.0)
    # Navigate: Help(0), NewGame(1), Statistics(2) - in main menu context
    ctl.timed_key('Down', after=0.5)
    ctl.timed_key('Down', after=0.5)
    ctl.timed_key('Down', after=0.5)
    enter(after=3.0)
    ss("17_statistics.png")
    ctl.timed_key('F4', after=2.0)

    # ========== HELP SCREEN ==========
    print("  [17] Help screen...", flush=True)
    ctl.timed_key('F1', after=2.0)
    enter(after=3.0)  # First item is Help
    ss("18_help.png")
    ctl.timed_key('F4', after=2.0)

    # ========== FINAL MAIN MENU ==========
    print("  [18] Final main menu...", flush=True)
    ss("19_final_menu.png")

    print("=== Done! 19 screenshots captured ===", flush=True)


def main():
    parser = argparse.ArgumentParser(description="Renode automation for Precursor apps")
    parser.add_argument('--init', action='store_true', help='Initialize PDDB from blank flash')
    parser.add_argument('--app', type=str, help='App name to launch and capture')
    parser.add_argument('--pin', type=str, default=DEFAULT_PIN, help='PDDB PIN (default: a)')
    parser.add_argument('--app-index', type=int, default=1, help='App position in submenu (default: 1)')
    parser.add_argument('--screenshots', type=str, help='Screenshot output directory')
    parser.add_argument('--xous-root', type=str, default=XOUS_ROOT)
    parser.add_argument('--script', type=str, help='Custom capture script (Python file with run(ctl, dir) function)')
    args = parser.parse_args()

    if args.init:
        reset_flash(args.xous_root)

    proc = start_renode(args.xous_root)
    ctl = RenodeController()
    ctl.connect()
    ctl._send('start')
    time.sleep(1)

    try:
        # Boot wait
        boot_time = 60 if args.init else 45
        print(f"Waiting {boot_time}s for boot...", flush=True)
        time.sleep(boot_time)

        # PDDB setup
        if args.init:
            ctl.init_pddb(args.pin)
        else:
            ctl.unlock_pddb(args.pin)

        # App launch
        if args.app:
            ctl.launch_app(args.app_index)

            # App-specific captures
            screenshot_dir = args.screenshots or f"/Volumes/PlexLaCie/Dev/Precursor/precursor-{args.app}/screenshots"

            # Custom capture script support
            if args.script:
                import importlib.util
                spec = importlib.util.spec_from_file_location("custom_captures", args.script)
                custom = importlib.util.module_from_spec(spec)
                spec.loader.exec_module(custom)
                custom.run(ctl, screenshot_dir)
            elif args.app == 'flashcards':
                capture_flashcards(ctl, screenshot_dir)
            elif args.app == 'timers':
                capture_timers(ctl, screenshot_dir)
            elif args.app == 'writer':
                capture_writer(ctl, screenshot_dir)
            elif args.app == 'c64':
                capture_c64(ctl, screenshot_dir)
            elif args.app == 'othello':
                capture_othello(ctl, screenshot_dir)
            elif args.app == 'calc':
                capture_calc(ctl, screenshot_dir)
            elif args.app == 'carse':
                capture_carse(ctl, screenshot_dir)
            else:
                print(f"No capture sequence defined for '{args.app}'. Taking one screenshot.", flush=True)
                ctl.screenshot(os.path.join(screenshot_dir, "app.png"))
    finally:
        ctl.quit()
        proc.terminate()
        try:
            proc.wait(timeout=5)
        except Exception:
            proc.kill()


if __name__ == '__main__':
    main()
