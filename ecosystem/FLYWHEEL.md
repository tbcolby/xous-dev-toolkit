# The Precursor app flywheel

Goal: make Precursor a device worth owning, with apps that work, look good and are honest about what they do, while each piece of work makes the next one cheaper.

```
        ┌────────────► 1. Audit ─────────────┐
        │   (apps vs gate; toolkit vs docs)   │
        │                                     ▼
 5. Measure                          2. Build / fix
 (status.json, CI,                   (Sonnet workers, brief +
  per-round log)                      LESSONS + reference apps)
        ▲                                     │
        │                                     ▼
 4. Upstream into toolkit  ◄──────── 3. Verify + retro
 (appkit helper, lint rule,          (orchestrator re-runs --grade,
  template, CLAUDE.md, LESSONS)       reviews vs audit; worker retro)
```

## The cycle

1. **Audit.** Find the gap between what is and what's good:
   - Apps: `bin/integrate --grade --json ecosystem/status.json <all repos>`, plus the per-app audit sections.
   - Toolkit: `ecosystem/DOCS_AUDIT.md`, i.e. the toolkit compared with the Xous Book and xous-core source. Re-run it when `XOUS_CORE_ANCHOR` moves or every ~10 rounds.
   - Owner experience: does the app do something worth opening on a Precursor? Is the README showing real screenshots?
2. **Build or fix.** Workers (Sonnet) take 2–4 apps each, using `REMEDIATION.md`, `LESSONS.md` and the reference apps. They never edit the toolkit.
3. **Verify and retro.**
   - The orchestrator re-runs `--grade` and reviews the diff against the audit section. A worker's claim counts as a claim, not as a result.
   - Every worker report ends with a Retro: what cost time, what doc was wrong, what got re-implemented, and one concrete suggestion.
4. **Upstream.** Each retro item becomes, in order of preference:
   1. a **mechanism**: an appkit or appkit-core helper, a lint rule, a template change, or a CI check;
   2. a **rule** in CLAUDE.md;
   3. a **lesson** in LESSONS.md.
   
   A lesson without a mechanism stays on the "Open" list until one exists. Toolkit edits happen in a `git worktree`, are graded against a spare xous-core tree, then fast-forwarded.
5. **Measure.** Record each round in the LESSONS log: apps finished, first-submission gate pass rate, minutes and tool calls per app, and the toolkit changes that came out of it. CI runs nightly against upstream xous-core, so drift shows up the day it lands.

## What "fantastic to own" adds beyond 10/10

The 10/10 gate is the floor. Once an app is past it, the next turns go to:

- **Proof it runs.** A Renode `capture.py` run that exits 0, with screenshots in the README. Hardware notes come from Tyler.
- **Polish.**
  - Consistent key hints, with Back on Backspace/F4 everywhere.
  - A first-run help screen.
  - Layouts that work on the real canvas.
  - Startup under one second, with no full redraw per key where it matters.
- **Coherence across apps.** One look (appkit `Screen` header/footer/list), one storage behaviour (basis-aware, never silently overwrites), one honesty standard in READMEs.
- **Localization.** Every string has five locales. Real translations are welcome; English fallbacks are the minimum.
- **Fewer, better apps.** Archive apps that aren't worth owning instead of carrying them. Propose new ones only where Precursor's strengths count: offline, private, a secure enclave, a keyboard, a long-lasting battery.

## Guardrails

- At most two concurrent builds on this machine, plus serialized orchestrator builds.
- Each worker gets a private xous-core tree and `CARGO_HOME`.
- Never push an app to `main` that fails the gate. Partial work goes to `remediation-wip`.
- Commits build on their own. Messages explain why. Attribution follows the session's rules.
- Ask Tyler before anything outward-facing beyond pushing his own repos: releases, new repos, external posts, or deleting or archiving an app.
