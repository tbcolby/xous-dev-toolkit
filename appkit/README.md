# precursor-appkit

Shared building blocks for Precursor apps on Xous. Lives at `xous-core/libs/precursor-appkit/` once `bin/integrate` has run; depend on it with:

```toml
precursor-appkit = { path = "../../libs/precursor-appkit" }
```

## Modules

### `keys`
Real key codes and a `Key` enum. `decode_rawkeys([k1, k2, k3, k4])` turns a GAM raw-keys message into keys, dropping padding and shift controls. `Key::is_back()` is Backspace or F4, never `∴` (GAM eats it).

### `app`
`register(&xns, sid, gam::APP_NAME_<KEY>, UxType::Framebuffer, Opcodes { .. })` registers the UX, gets the content canvas, and panics with an explanation if GAM rejects the name. `focus_state(code)`, `leave(&screen)` (relinquish focus), `shutdown(&xns, sid)`.

### `screen`
`Screen` wraps the GAM handle and canvas: `clear`, `fill`, `frame`, `line`, `pixel`, `text`, `text_in`, `text_centered`, `header(title) -> content_top`, `footer(hint) -> content_bottom`, `list(top, bottom, items, &ListState) -> visible_rows`, `flush`. Size comes from the canvas; selection uses a border and `▸`, not `invert`.

### `layout`
`ListState { selected, top }` with `step`, `step_wrapping`, `clamp`, `visible_range`; `rows_that_fit`. Pure and unit-tested.

### `store`
`Store::open(dict)` (waits for PDDB mount). `load::<T>(key) -> Result<Option<T>>`, `load_or_default`, `save(key, &T)`, `delete`, `keys`, `basis_of`, `target_basis`. Saves delete-then-rewrite in the key's own basis and `sync()`. `watch_bases(cid, opcode)` sends your app a message whenever a basis is locked or unlocked; reload on it.

### `rng`
`Trng::new(&xns)`: `below(n)`, `range_inclusive(lo, hi)`, `shuffle`, `fill_bytes`, all unbiased and **panicking on TRNG failure**. `try_u32()` if you'd rather show an error. The algorithms are generic over `RandomSource` for host tests.

### `pump`
`Pump::spawn(main_cid, opcode, interval_ms)`, then `run()` on foreground, `stop()` on background, `quit()` on exit. Ticks arrive as blocking scalars; answer each with `pump::ack(&msg)`. Generation counting stops double-ticking after rapid focus changes.

## Tests

```bash
cd $XOUS_ROOT && cargo test -p precursor-appkit --no-default-features   # 19 host tests
```

The Xous-dependent modules are compiled by CI through the scaffolded app (`bin/new-app` + `bin/integrate --check`).

## Not yet covered

Modals wrappers, text entry, networking, and a status-bar-aware vibration/notification helper. Use the raw `modals`/`net` APIs per `agents/graphics.md` and `agents/networking.md`.
