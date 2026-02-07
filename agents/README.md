# Precursor Development Agent System

This directory contains specialized sub-agents for different phases of Precursor/Xous application development. Each agent has deep expertise in its domain and follows consistent handoff protocols.

## Agent Roster

| Agent | Domain | When to Invoke |
|-------|--------|----------------|
| **Supervisor** | Orchestration | Route tasks, coordinate handoffs, track progress |
| **Ideation** | Concept & UX | New app ideas, feature design, UX patterns |
| **Architecture** | Code Structure | State machines, message passing, multi-threading |
| **Graphics** | Display & UI | GAM API, TextView, drawing, 1-bit optimization |
| **Storage** | Persistence | PDDB patterns, serialization, key management |
| **Networking** | Connectivity | TCP/UDP, TLS, HTTP, background threads |
| **Build** | Toolchain | cargo xtask, manifest, workspace, deployment |
| **Testing** | QA & Automation | Renode scripts, screenshots, validation |
| **Review** | Quality | Code review, patterns, performance, pitfalls |
| **System** | Hardware | COM service, battery, sensors, WiFi hardware, backlight, power |

## Agent Invocation

To load an agent's context, read `agents/<name>.md` when entering that domain. Switch agents:
- At phase boundaries (e.g., Architecture done, starting Graphics)
- When blocked and needing a different specialist
- When the user's question falls outside current agent's domain

## Invocation Pattern

Agents are invoked by loading their context file. The supervisor routes based on task type:

```
User Request
     │
     ▼
┌─────────────┐
│ Supervisor  │ ◄── Routes to appropriate specialist
└─────────────┘
     │
     ├──► Ideation ──► Architecture ──► Graphics/Storage/Networking
     │                                            │
     │                                            ▼
     │                                        Build ──► Testing
     │                                                      │
     └──────────────────────────────────────────────────────┘
                                                      Review

```

## Handoff Protocol

When an agent completes its phase, it produces a structured handoff:

```markdown
## Handoff: [Source Agent] → [Target Agent]

### Completed
- [What was accomplished]

### Artifacts
- [Files created/modified]

### Next Steps
- [Specific tasks for next agent]

### Decisions Made
- [Key choices that constrain future work]

### Open Questions
- [Issues requiring user input or further investigation]
```

## Usage Examples

### New App Development
```
1. Supervisor receives "Build a weather app"
2. → Ideation: Define features, UX flow, hardware constraints
3. → Architecture: Design state machine, message types, modules
4. → Graphics + Networking (parallel): UI components + API client
5. → Storage: Settings persistence, cache strategy
6. → Build: Cargo.toml, manifest.json, workspace integration
7. → Testing: Renode capture sequence, validation screenshots
8. → Review: Final code review, performance check
```

### Bug Fix
```
1. Supervisor receives "App crashes on key press"
2. → Review: Diagnose issue, identify root cause
3. → [Relevant specialist]: Implement fix
4. → Testing: Verify fix, regression check
```

### Feature Addition
```
1. Supervisor receives "Add export to existing app"
2. → Ideation: Define export UX, format options
3. → Architecture: Extend state machine, new messages
4. → [Implementation agents as needed]
5. → Testing: Capture new screenshots
6. → Review: Verify integration
```

## Decision Trees

When unsure which agent to use:

**"Is this Graphics or Architecture?"**
- What to draw, how it looks → **Graphics**
- When/how data flows, state transitions → **Architecture**

**"Is this Storage or Networking?"**
- Persisting data locally → **Storage**
- Fetching data from remote → **Networking**
- Caching fetched data locally → **Both** (Networking fetches, Storage caches)

**"Is this Networking or System?"**
- TCP/HTTP/TLS protocol work → **Networking**
- WiFi hardware on/off, scanning → **System**
- Battery-aware poll intervals → **System** (informs Networking strategy)

**"Is this Graphics or System?"**
- Drawing UI elements → **Graphics**
- Backlight brightness, haptic vibration → **System**

## Agent Files

Each agent file (`<name>.md`) contains:
1. **Role** - What this agent does
2. **Expertise** - Domain knowledge and APIs
3. **Patterns** - Common solutions and templates
4. **Quality Criteria** - How to evaluate success
5. **Handoffs** - What to pass to next agents
