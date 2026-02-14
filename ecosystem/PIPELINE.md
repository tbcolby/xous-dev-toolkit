# DevOps Pipeline — Per-App Build Process

Every app in the 32-app ecosystem follows this 8-stage pipeline.
Stages 1-7 build the app. Stage 8 evolves the agent architecture.

---

## Stage 1: IDEATION (agents/ideation.md)

**Input**: App concept from the master list
**Output**: DESIGN.md with complete specification

Tasks:
1. Define the problem statement and Precursor fit
2. MoSCoW feature prioritization (Must/Should/Could/Won't)
3. Screen flow diagram (state transitions)
4. Keyboard mapping (using STANDARDS.md conventions)
5. PDDB schema design (dictionaries, keys, serialization)
6. TCP port assignment (from STANDARDS.md allocation)
7. Complexity estimate (LOC, modules, threading needs)

**Quality gate**: DESIGN.md reviewed against ideation.md patterns.

---

## Stage 2: ARCHITECTURE (agents/architecture.md)

**Input**: DESIGN.md
**Output**: Module plan, state machine, opcode design

Tasks:
1. Define AppState enum (all states + transitions)
2. Define AppOp enum (following opcode conventions)
3. Plan module decomposition (single-crate vs library extraction)
4. Identify threading needs (pump thread? network thread?)
5. Define data structures (with RV32 constraints — no AtomicU64)
6. Plan error handling strategy
7. Estimate memory budget

**Quality gate**: Architecture reviewed against architecture.md patterns.

---

## Stage 3: IMPLEMENTATION (parallel specialist agents)

### Stage 3a: Graphics (agents/graphics.md)
- Implement drawing functions
- Screen layout with header/content/footer
- Text rendering with appropriate fonts
- Redraw cycle (clear → draw → gam.redraw())
- Modal dialogs if needed

### Stage 3b: Storage (agents/storage.md)
- PDDB dictionary/key creation
- Settings load/save with serde JSON
- Data collection CRUD operations
- Migration version tracking

### Stage 3c: Core Logic
- State machine implementation
- Business logic (algorithms, parsers, engines)
- Input handling (keymap, mode switching)

### Stage 3d: Networking (agents/networking.md) — if applicable
- TCP import/export on assigned port
- Background thread with message notification
- Offline-first design

**Quality gate**: Each module compiles independently, follows standards.

---

## Stage 4: INTEGRATION (agents/build.md)

**Input**: All implemented modules
**Output**: Compiling, registered app

Tasks:
1. Assemble Cargo.toml with pinned dependency versions
2. Wire main.rs event loop (message dispatch)
3. Create manifest.json entry
4. Verify workspace integration paths
5. Build: `cargo build -p {name} --target riscv32imac-unknown-xous-elf`
6. Fix all compiler warnings

**Quality gate**: Clean build with zero warnings.

---

## Stage 5: TESTING (agents/testing.md)

**Input**: Built app
**Output**: Screenshots, capture script, validation

Tasks:
1. Write capture function for renode_capture.py
2. Run through all app states with automation
3. Capture numbered screenshots (01_initial.png, 02_..., etc.)
4. Validate key interactions work
5. Test edge cases (empty state, full storage, rapid input)
6. Verify focus change behavior (background/foreground)

**Quality gate**: All screenshots captured, all states reachable.

---

## Stage 6: REVIEW (agents/review.md)

**Input**: Complete app
**Output**: Review report, fixes applied

Checklist:
1. Standards compliance (STANDARDS.md full checklist)
2. Pattern compliance (architecture.md, graphics.md patterns)
3. Performance review (no busy-wait, proper pump rates)
4. Memory efficiency (no large stack allocations)
5. Battery considerations (stop work when backgrounded)
6. Security review (input validation, no hardcoded secrets)
7. Thread safety (proper Arc/Mutex usage)

**Quality gate**: All review items pass.

---

## Stage 7: DOCUMENTATION

**Input**: Reviewed app
**Output**: README.md, CLAUDE.md, screenshots

Tasks:
1. Write README.md following the 12-section standard
2. Write CLAUDE.md with app-specific agent instructions
3. Organize screenshots with descriptive names
4. Write manifest integration instructions
5. Document any workarounds or discoveries

**Quality gate**: README complete, screenshots match current state.

---

## Stage 8: AGENT EVOLUTION

**THE MOST IMPORTANT STAGE.**

After each app ships, we evolve the agent architecture:

### 8a: Capture Learnings
- What patterns worked well?
- What was painful or error-prone?
- What knowledge was missing from existing agents?
- What could be automated?

### 8b: Evolve Existing Agents
- Update toolkit agents (graphics.md, storage.md, etc.) with new patterns
- Add to discoveries.md with debugging insights
- Update STANDARDS.md if conventions need refinement

### 8c: Create Specialist Agents
New specialist agents born from specific app types:

| App Type | Potential Specialist Agent |
|----------|--------------------------|
| QR/Barcode | `agents/encoding.md` — barcode/QR encoding patterns |
| Crypto apps | `agents/crypto.md` — cipher implementations, key management |
| Visual/Art apps | `agents/framebuffer-art.md` — parametric curves, fractals, dithering |
| Game apps | `agents/game-engine.md` — grid games, AI opponents, scoring |
| Productivity apps | `agents/productivity.md` — CRUD patterns, list management |
| Network apps | `agents/protocol.md` — RSS/HTTP parsing, data formats |
| Emulator apps | `agents/emulator.md` — CPU emulation, instruction decode, memory mapping |
| Simulation apps | `agents/simulation.md` — grid simulations, physics, cellular automata |

### 8d: Update Automation
- Add capture function to renode_capture.py
- Add any new renode_lib.py utilities discovered
- Update test patterns

### 8e: Write AGENTS.md
Each app repo gets an AGENTS.md documenting:
- Which toolkit agents were used and how
- What specialist knowledge was needed
- New agent definitions that emerged
- Recommendations for similar future apps

---

## Pipeline Tracking

For each app, track stage completion:

```
App: precursor-{name}
[ ] Stage 1: Ideation      → DESIGN.md
[ ] Stage 2: Architecture   → Module plan
[ ] Stage 3: Implementation → Source code
[ ] Stage 4: Integration    → Clean build
[ ] Stage 5: Testing        → Screenshots
[ ] Stage 6: Review         → All checks pass
[ ] Stage 7: Documentation  → README + CLAUDE.md
[ ] Stage 8: Agent Evolution → Toolkit updated
```
