# doc-snippets: compile check for the knowledge docs

This crate is the proof that the Rust in `agents/*.md`, `discoveries.md` and
`ecosystem/STANDARDS.md` is real API. Every fenced `rust` block in those files is reproduced here
(as functions in modules, with minimal scaffolding such as stub types); blocks that are
deliberately illustrative are fenced `rust,ignore` in the docs, with the reason next to them.

Anchor: xous-core **31fbe23** (`XOUS_CORE_ANCHOR`). The only systematic substitution is the app
name: docs say `gam::APP_NAME_MYAPP`, this crate uses `gam::APP_NAME_DOCSNIP`.

| Module | Doc |
|---|---|
| `main.rs` | architecture.md "App Lifecycle" (verbatim, it is the crate's `main`) |
| `arch.rs` | architecture.md (pump, threads, IPC, memory messages, heap, command prefix) |
| `gfx.rs` | graphics.md (primitives, TextView, header/selection without invert, modals) |
| `storage.rs` | storage.md (read, delete-then-recreate `store()`, bases, bulk, migration) |
| `system.rs` | system.md (COM, NetManager, WiFi state memory messages) |
| `net_snips.rs` | networking.md (std::net, ureq + `tls::xtls::TlsConnector`) |
| `rng.rs` | randomness.md (fail-closed TRNG) |
| `encoding.rs` | encoding.md (auto-fit, batched module rendering) |
| `keys.rs` | STANDARDS.md section 1 key constants |
| `review.rs` | review.md / discoveries.md / STANDARDS.md registration and focus |
| `imports.rs` | import lines and tiny blocks quoted in the docs |

## Run it

```bash
cd $XOUS_ROOT                                   # xous-core checked out at the anchor
cp -R /path/to/xous-dev-toolkit/tests/doc-snippets apps/docsnip
# add "apps/docsnip" to [workspace] members (NOT default-members) in Cargo.toml, and to
# apps/manifest.json:
#   "docsnip": { "context_name": "Doc Snippets",
#                "menu_name": { "appmenu.docsnip": { "en": "Doc Snippets", "en-tts": "Doc Snippets",
#                  "fr": "Doc Snippets *EN*", "ja": "Doc Snippets *EN*", "zh": "Doc Snippets *EN*" } } }
cargo xtask hosted-ci docsnip                   # first build ~5 min; must finish with no errors
```

The paths in `Cargo.toml` (`../../services/...`, `../../libs/...`) assume the crate sits at
`xous-core/apps/docsnip`. Lint-level cleanliness is not the goal (`#![allow(dead_code, ...)]`);
compiling is.

When you add or change a Rust block in the docs, mirror it here and rerun the build. A snippet
that can't compile (pseudo-code, `...` elisions, wrong/right pairs) must be fenced `rust,ignore`
with a one-line reason.
