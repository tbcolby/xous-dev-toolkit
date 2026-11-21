# Slice report: `generative`

Repos: precursor-flames, -penrose, -kaleidoscope, -sacred, -moire, -lissajous, -spirograph, -reaction, -life, -death
(all single-commit repos dated 2026-02-13; 4,868 non-blank/non-comment Rust lines in total; zero tests).

Paths below are relative to `SCRATCH/precursor/`. Every `.rs` file in the slice was read. Numeric claims marked
"(measured)" come from a host harness that `#[path]`-includes the apps' pure modules unmodified
(`SCRATCH/gen_harness/src/main.rs`; the only patch is the one-token cast needed to make spirograph `math.rs` compile).

## Executive summary

| # | Finding | Apps | Severity |
|---|---------|------|----------|
| 1 | The shared "parabolic" sine is wrong: it returns **0 at 90 deg** and 1.0 at 45 deg, so `(cos, sin)` is `(0,0)` at 0/90/180/270 deg and `(1,1)` at 45 deg. Circles render as an X, rotations by 0 collapse to the centre. 8 copies of the same function. | penrose, kaleidoscope, sacred, moire, lissajous, spirograph | Critical |
| 2 | Arrow keys use Apple PUA codes `U+F700..F703`; Precursor emits `U+2190..2193` (the toolkit README explicitly says "Do NOT use Apple PUA codes"). All arrow navigation is dead. | all 10 | Critical |
| 3 | `KEY_MENU` (`U+2234`) is consumed by GAM to raise the main menu before it reaches an app context (`services/gam/src/contexts.rs:741`), so every "Menu: quit/back" binding never fires. Life/Death `Running` can then only be left via `E` then Backspace back to Running: there is no path back to the selector. | all 10 | High |
| 4 | The GAM/ticktimer/xous API used does not exist in the tree and is not the API the toolkit documents (no content canvas is ever requested; `gam.clear()`, `gam::RawkeysEvent`, `Ticktimer::set_alarm`, `xous::wait_thread(Duration)`). 29-64 compile errors per app. Suspected never compiled, not drift (see "Upstream drift"). | all 10 | Critical (build) |
| 5 | Penrose draws a single pixel: coordinates are pixel-scale but `edges_to_screen` divides by 1000 (measured: 480 edges, 1 distinct, all `(168,260)-(168,260)`). Independently, the obtuse-triangle subdivision is not a Robinson subdivision, so the result would not be a Penrose tiling even when scaled. | penrose | Critical |
| 6 | Tick timers never stop: reaction/life/death re-arm unconditionally, ignore focus state, and keep simulating and drawing in the background. Reaction is CPU-bound (estimated 0.5-3 s of compute per 500 ms tick). | reaction, life, death | High |

No floating point is used anywhere in the slice (grep for `f32`/`f64`/float literals: none). The no-FPU concern is
therefore moot; the real hot-loop costs are 64-bit multiplies, non-constant `%`, per-step heap allocation, and above
all one blocking IPC per drawn primitive.

Per-app scores: flames 3, penrose 1.5, kaleidoscope 2, sacred 2, moire 2.5, lissajous 2, spirograph 2, reaction 3,
life 4, death 4. The scores are low because none of the apps can build or be navigated; the pure logic in life, death,
flames and reaction is in much better shape than the trig-dependent apps.

---

## Issues common to all 10 apps (listed once, referenced per app as C1..C9)

| ID | Severity | Where | Description / failure scenario | Fix |
|----|----------|-------|--------------------------------|-----|
| C1 | Critical | `*/src/app.rs` key consts (e.g. `precursor-flames/src/app.rs:14-17`, `precursor-life/src/app.rs:7-10`) | `KEY_UP/DOWN/LEFT/RIGHT = U+F700..F703`. Precursor keyboard sends `U+2191/2193/2190/2192` (`xous-dev-toolkit/README.md:221-228`, `ecosystem/STANDARDS.md:40`). No menu cursor, parameter, editor cursor or speed control works on hardware or Renode. | Use `'\u{2190}'..'\u{2193}'` (or the glyph literals). |
| C2 | High | `*/src/app.rs` `KEY_MENU = '\u{2234}'` | GAM eats `'∴'` for App-layout contexts and raises the main menu (`xous-core/services/gam/src/contexts.rs:741-759`). "Menu: quit" / "Menu: back" never arrive. Combined with C1 the only working keys are Enter, Space, Backspace and letters. | Bind back/quit to a deliverable key (e.g. `q`, Backspace, or Left once C1 is fixed) or add entries to the app's GAM menu. |
| C3 | High | `*/src/main.rs` exit path (flames `main.rs:62-66,84-85`; life `main.rs:34,47-48`) | "Quit" calls `relinquish_focus`, unregisters the server and `terminate_process(0)`. Xous apps are resident processes started at boot; after quitting, the app-menu entry points at a dead context until reboot. Currently masked by C2. | Never terminate; on "back" at top level just `gam.relinquish_focus()`/raise the main menu and keep the loop alive. Drop the dead `Quit = 255` opcode. |
| C4 | Critical (build) | `*/src/main.rs:1-2` | `#![cfg_attr(target_os = "xous", no_std)]` + `no_main`. On the real target `target_os` is `"xous"`, so this activates `no_std`/`no_main` with no `#[panic_handler]`, no global allocator (yet `extern crate alloc` is used) and no entry point. Upstream apps and the toolkit checklist (`xous-dev-toolkit/agents/review.md:16-17`) use `target_os = "none"`. Hosted lint does not exercise this branch. | Use `target_os = "none"` like upstream (Xous has `std`). |
| C5 | Critical (build) | `*/src/ui.rs`, `*/src/main.rs` | No content canvas: `register_ux` token is bound to `_token` and discarded; `request_content_canvas` is never called; every draw call omits the `Gid`. `gam.clear()`, `Gam::draw_line(Line)`, `draw_rectangle(Point, Point)`, `TextView::new(bounds)`, `TextBounds::BoundingBox(Point, Point)`, `gam::RawkeysEvent`, `xous::String::<128>::from_str`, `xous::wait_thread(Duration)` have no counterpart in the tree. | See "Upstream drift" for the fix pattern. |
| C6 | Medium | `*/src/main.rs` `FocusChange` arm (flames `main.rs:69-71`) | Focus messages are not decoded (`gam::FocusState::convert_focus_change`); the handler just sets `needs_redraw`, so the app redraws when it is being backgrounded and has no `allow_redraw` gate. | Track foreground/background as `apps/ball` does; draw only in foreground. |
| C7 | Medium | `*/src/main.rs` `ux_type: gam::UxType::Chat` | Chat layout reserves an input/IME area and expects a predictor; these are raw-key framebuffer apps. Layout constants (`FOOTER_Y = 490`, 536-px assumptions) are hard-coded instead of read from `get_canvas_bounds`. | `UxType::Framebuffer`; derive geometry from canvas bounds. |
| C8 | Medium | every `draw_*` viewing function | One GAM call per primitive; each real GAM draw call is a blocking `Buffer::lend` (page-mapped IPC) that GAM then forwards to the graphics server. Measured primitive counts per redraw: moire up to 15,436 horizontal runs ("Concentric Ripple"), lissajous/spirograph up to 16,383 segments, flames tens of thousands of runs at 200K iterations, life/death roughly 1,800 rectangles at 25% random fill. Expect multi-second redraws, repeated on every `Redraw`/`FocusChange` because nothing is cached GAM-side. | Batch with `GamObjectList` (32 objects per IPC, `gam::draw_list`) at minimum; for bitmaps use `draw_bitmap`/tiles (gam `ditherpunk` feature) so a full 336x456 frame is a handful of lends. |
| C9 | Low | `*/Cargo.toml`, repo layout | `xous-ipc` and `ticktimer-server` declared but unused in 7 apps; life/death pin different versions (`xous 0.9.64`, `num-derive 0.3`) from the other eight (`0.9.69`, `0.4.2`); `[features] default = []` noise; no manifest.json snippet or install steps in any README; `APP_NAME` is a local string rather than `gam::APP_NAME_*`, so the context name must be kept in sync with `apps/manifest.json` by hand. No unit tests although every algorithm module is pure and host-testable (AGENTS.md even says "testable without Xous"). | One workspace-level version set; README install section; tests (see cross-cutting). |

All explicit `unwrap()`/`expect()` in the slice are on startup IPC (name registration, GAM, TRNG) plus
`xous::receive_message(sid).unwrap()` and `gam.relinquish_focus().unwrap()` in the loop; no parse/PDDB paths exist.
None of the apps uses PDDB, modals or networking, so there is no secret-handling or plausible-deniability surface.
RNG use is cosmetic only (LCG seeded from TRNG or uptime), which is fine.

---

## Upstream drift (applies to all 10; not repeated per app)

Error distribution (from `SCRATCH/lint/<pkg>.txt`): 3-5 errors in `main.rs`, 26-61 in `ui.rs`, plus one genuine
type error in spirograph `math.rs`. Everything in `ui.rs` is the same handful of mistakes repeated per call site.

What is plausibly drift:
- `Point`/`Line` coordinates are `isize` now (were `i16`): all `i16` layout math needs widening.
- `UxRegistration.app_name` is now `String` (was `xous_ipc::String<128>`).

What is not explained by drift (Suspected: these apps were generated against an imagined API and never compiled):
- No canvas `Gid` anywhere, and no `request_content_canvas`. GAM has been canvas/token based throughout; the toolkit's
  own examples use `gam.draw_rectangle(gid, Rectangle::new_with_style(..))` and `TextView::new(gid, ..)`.
- `gam.clear()`, `gam::RawkeysEvent::from_message(..).char()`, `xous::wait_thread(Duration)` (a real syscall but it
  takes a thread wait handle), `xous::String` (the type lived in `xous_ipc`).
- `Ticktimer::set_alarm` is called with two different signatures: `set_alarm(sid, op as u32, 500)` in
  `precursor-reaction/src/main.rs:52,78` versus `set_alarm(sid, ms as i64, op as usize, false)` in
  `precursor-life/src/main.rs:25,40` and `precursor-death/src/main.rs:25,40`. An API cannot have drifted into both.
- `precursor-spirograph/src/math.rs:23` returns `i64` from a function declared `-> i32` (pure Rust, no Xous involved).
- Keys and `cfg_attr` contradict the toolkit's own documented rules (C1, C4).

I could not check older xous-core history (the tree is a one-commit shallow clone), hence "Suspected". Tyler's
working copies may differ from GitHub.

Fix pattern (identical for every app, which is the main argument for the shared crate below):
1. `register_ux` with `UxType::Framebuffer`, keep the token, `gid = gam.request_content_canvas(token)`,
   `bounds = gam.get_canvas_bounds(gid)`.
2. Clear with `gam.draw_rectangle(gid, Rectangle::new_with_style(Point::new(0,0), bounds, DrawStyle{fill: Some(White), ..}))`.
3. Text via `TextView::new(gid, TextBounds::BoundingBox(Rectangle::new(..)))` and `write!(tv.text, ..)`.
4. Raw keys: decode the four scalar args with `core::char::from_u32` (see `apps/ball/src/main.rs:79-86`).
5. Periodic work: a pump thread with `ticktimer.sleep_ms()` sending a blocking scalar, stopped on
   `FocusState::Background` (`apps/ball` pattern).

---

## flames ("Fractal Flames")

Verdict: clean fixed-point chaos game, but the viewport is hard-wired to the unit square so 4 of 8 presets are
clipped or invisible, and the name over-promises. **Score 3/10.**

### Bugs
| Sev | file:line | Description | Failure scenario | Fix |
|-----|-----------|-------------|------------------|-----|
| High | `precursor-flames/src/ifs.rs:157-160` | Screen mapping assumes the attractor lies in `[0,1]x[0,1]`. | Measured at 50K iterations: Barnsley Fern bbox is x[-2.18,2.65] y[0.11,10.0], only 0.8% of points land on screen (323 pixels); Dragon Curve 57.6% on screen; Tree 48.3%; Maple Leaf 95.9%. The headline preset shows a few specks. | Store a per-preset bounding box (or run a short pre-pass to measure it) and map bbox to the render area. |
| Medium | `precursor-flames/src/ifs.rs:159-160` | x is scaled by 336 and y by 456 independently. | Sierpinski triangle, carpet and Koch curve are stretched 36% vertically; the "square" carpet is a rectangle. | Uniform scale = min(w/bw, h/bh), centre the result. |
| Medium | `precursor-flames/src/app.rs:91-93,105-113` with `ifs.rs:134` | Render runs synchronously in the key handler; up to 200K iterations each with four `i64` multiplies and a weight scan. | UI is unresponsive for the duration (rough estimate 0.3-1 s at 100 MHz), then C8 adds a redraw of tens of thousands of line IPCs. Up/Down in the viewer re-renders from scratch each press. | Render in slices from a pump message (progressive display also looks better), keep accumulating into the same bitmap when only the iteration count grows. |
| Low | `precursor-flames/src/ifs.rs:137` | `total_weight` of 0 would divide by zero; `weight` comment says 0..1000 but nothing enforces the sum. | Not reachable with the shipped presets (all sum to 1000). | `debug_assert!(total_weight > 0)` or compile-time check. |
| Low | `precursor-flames/src/ifs.rs:43-45` | Doc comment says "millionths"/`fp_from`; the function takes thousandths and is `fpf`. | Reader confusion only. | Fix comment. |
| Low | `precursor-flames/src/app.rs:10` | Unused `alloc::format` import (compiler warning). | - | Remove. |
| -   | C1-C9 | Common issues. | | |

Also verified fine: bitmap indexing (`row_bytes = 42`, bit index `sy*width+sx`) is consistent between `ifs.rs:163-164`
and `ui.rs:102-123`; LCG high bits are used; fixed-point range is ample (max |coord| about 10.0 in 16.16).

### Improvements
1. Per-preset viewport (fixes the High bug) and uniform aspect.
2. Rename or implement: these are plain affine IFS, not fractal flames (no variations, no log-density). README says "Fractal Flames".
3. Blit the bitmap instead of run-length lines (C8); the README's "run-length bitmap display - efficient" claim does not hold for scattered point clouds, where almost every pixel is its own run.
4. "New seed" key in the viewer (TRNG is only sampled once at startup, `main.rs:49`).
5. Density-aware plotting (count hits, threshold or dither) would make 200K-iteration renders differ visibly from 50K on a 1-bit display.

---

## penrose ("Penrose Tiler")

Verdict: draws one pixel; the geometry is wrong at three independent levels (scale, trig, subdivision rule).
**Score 1.5/10.**

### Bugs
| Sev | file:line | Description | Failure scenario | Fix |
|-----|-----------|-------------|------------------|-----|
| Critical | `precursor-penrose/src/app.rs:77,93-94` with `tiling.rs:170-175` | Seeds are built with `radius = 200` (pixels) but `edges_to_screen` computes `coord * scale / 1000` with `scale = 1`. | Every vertex becomes 0, every edge is `(168,260)-(168,260)` (measured: level 4 gives 480 edges, 1 distinct). The screen shows a single dot at any level and seed. | Work in milli-pixels: `radius = 200_000`, keep `/1000` (products stay under 2^31: 400,000 x 618). |
| Critical | `precursor-penrose/src/tiling.rs:80-93` | Sine bug (see cross-cutting X1). | Measured decagon rim vertices: `(0,0), (129,129), (129,129), (-192,192), ...`; two of ten coincide with the centre and the rest sit on the diagonals. | Shared correct trig. |
| High | `precursor-penrose/src/tiling.rs:56-73` | Obtuse-triangle case splits into 2 children by the line Q-C. A Robinson obtuse triangle must split into 3 (P3) or the acute one into 3 (P2); one of the two types always yields three. | Verified numerically: starting from an exact golden triangle, the children of the code's obtuse split have side ratios (1, 2.62, 3.08) and (1, 1.90, 2.62) instead of (1, 1, 1.618)/(1, 1.618, 1.618). Output is not a Penrose tiling; the title's count `10 * 2^level` is the tell (a real one grows by about phi^2 per level). | Use the standard P3 rule: for obtuse (A,B,C): `Q = B+(A-B)/phi`, `R = B+(C-B)/phi`, emit `(obtuse R,C,A)`, `(obtuse Q,R,B)`, `(acute R,Q,A)`. The acute case at `tiling.rs:39-55` already matches that scheme. |
| High | `precursor-penrose/src/tiling.rs:134-162` | "Star" seed emits each sector twice with swapped B/C (two coincident triangles) and tags them obtuse, but the triangle centre-rim-rim with a 72 deg apex is neither Robinson shape (side ratios 1 : 1 : 1.176). | Even with the other bugs fixed, the star seed produces overlapping garbage. | Build the star from 10 acute triangles with a 36 deg apex at the rim points, or drop the option. |
| Medium | `precursor-penrose/src/tiling.rs:1-6`, README | Module and commit say "P2 (kite and dart)"; the acute rule implemented is the P3 (rhomb) deflation. | Documentation/behaviour mismatch. | Decide on P3, fix docs. |
| Medium | `precursor-penrose/src/tiling.rs:177-179` | All three edges of every half-tile are drawn, including the edge shared by the two halves of a rhomb, and every interior edge is emitted twice. | Shows a triangle mesh, not rhombs; doubles the IPC count. | Skip the A-opposite (base) edge; dedupe with a sorted edge set. |
| Medium | `precursor-penrose/src/ui.rs:107-108` | Whole segment dropped if either endpoint is outside the content box. | Tiles at the rim vanish rather than being clipped. | Let the canvas clip, or clip properly (X4). |
| Low | `precursor-penrose/src/tiling.rs:23,25,28-32` | phi as 1618/1000 with truncating integer lerp; error compounds per level. | Visible cracks at level 6-7 once coordinates are real. | 16.16 with `INV_PHI = 40504`, round to nearest. |
| Low | `precursor-penrose/src/app.rs:9` | Unused import. | - | Remove. |
| -   | C1-C9 | Common issues. | | |

### Improvements
1. Fix scale, trig and the subdivision rule together and add a host test: all triangles after N levels must have Robinson side ratios within tolerance and total area must be conserved.
2. With a correct rule level 7 is about 10 x phi^14, roughly 8,400 triangles (about 200 KB as 28-byte `Triangle`s through two generations): cap the level by heap and switch `Triangle` to `i32` pairs in a preallocated buffer, or cull triangles outside the viewport after each level.
3. Compute in a pump-driven slice; a level-7 subdivision in the key handler blocks input.

---

## kaleidoscope

Verdict: structure is fine; output is wrong because every rotation uses the broken trig. **Score 2/10.**

### Bugs
| Sev | file:line | Description | Failure scenario | Fix |
|-----|-----------|-------------|------------------|-----|
| Critical | `precursor-kaleidoscope/src/mirror.rs:19-36,39-47` | Sine bug (X1): `cos(0) = sin(0) = 0`. | Measured for the default "Snowflake" settings (6-fold, 12 shapes, seed 42): 85 of 144 segments collapse to the centre point; the k=0 "rotated copy" is always `(cx,cy)-(cx,cy)`; the rest are scaled/skewed by up to 1.41x. | Shared correct trig. |
| Medium | `precursor-kaleidoscope/src/mirror.rs:83,85` | Seed angles are always drawn from 0..128 units (45 deg) "for 8-fold" regardless of `folds`. | For 3-6 folds each wedge is only partly filled; for 12-16 folds seeds spill across wedges. Not a true kaleidoscope wedge except at 8. | `ang = a * (1024 / (2*folds)) / 256` (half-sector, since the mirror supplies the other half). |
| Medium | `precursor-kaleidoscope/src/mirror.rs:105` | `sector = 1024 / folds` truncates. | 6-fold uses 170 units (6 x 170 = 1020), 12-fold 85: a 1.4 deg seam; the worst case is 7/9/11/13 folds. | Compute each angle as `k * 1024 / folds`, or use a 4096+ unit circle. |
| Low | `precursor-kaleidoscope/src/mirror.rs:120-121` | `rotate_point(.., 0)` on the mirrored copy is an intended no-op (and currently the collapse path). | Wasted work; with X1 it zeroes the point. | Delete. |
| Low | `precursor-kaleidoscope/src/main.rs:63-66` | A TRNG IPC on every keypress to keep `trng_seed` fresh. | Harmless but wasteful; also means "Random" needs main.rs plumbing. | Fetch on demand when Random is chosen. |
| Low | `precursor-kaleidoscope/src/ui.rs:106-109` | Whole-segment rejection instead of clipping. | Only matters once geometry is right and radius grows. | X4. |
| Low | `precursor-kaleidoscope/src/app.rs:10` | Unused import. | - | Remove. |
| -   | C1-C9 | Common issues. | | |

### Improvements
1. Fix trig; add a symmetry test (output set invariant under rotation by one sector and under the mirror).
2. In `Viewing`, let a key re-roll (new TRNG seed) and Up/Down change folds without going back to the menu; `handle_viewing` currently only exits.
3. Richer seeds (arcs, polylines, filled dots) read much better than 12 short segments on a 1-bit panel.

---

## sacred ("Sacred Geometry")

Verdict: ten figures, but every circle is drawn as an X and several constructions are not the named figure.
**Score 2/10.**

### Bugs
| Sev | file:line | Description | Failure scenario | Fix |
|-----|-----------|-------------|------------------|-----|
| Critical | `precursor-sacred/src/geometry.rs:17-35`, duplicated in `ui.rs:149-180` | Sine bug (X1), present twice in this app. | Measured Seed of Life centres at r=120: `(168,260), (168,260), (274,366), (62,366), (168,260), (62,154), (274,154)`; three of seven circles sit on the centre and the 32-gon "circle" itself degenerates to diagonal strokes. Every figure in the app is affected. | Shared trig; a real midpoint circle or GAM `draw_circle`. |
| Medium | `precursor-sacred/src/ui.rs:107-108,138-139` | Segment rejected if either endpoint is off the content area. Default radius is 120 with centre x=168. | Seed of Life spans 2r = 240 px either side (needs r <= 83 to fit), Flower of Life 3r, Torus 2r: at the default size large parts of the figure are silently missing; Up (bigger) makes it worse, up to r=200. | Clip rather than reject; choose a per-figure default radius from its extent (r/2 for seed, r/3 for flower). |
| Medium | `precursor-sacred/src/geometry.rs:213-258` | Golden spiral: squares are advanced by the current square's side (`x += s`, `y -= s`, ...) without using the previous/next sizes. Tracing i=0..2: the 2k square is placed at `(x+k, y-k)` and overlaps the second unit square. Also `scale = r*1024/(55*1024) = r/55`, which is 0 for r < 55 and then every square is forced to 1 px (reachable: minimum radius is 40). | Not a Fibonacci tiling, arcs do not join into a spiral; at r=40/50 the figure is a few pixels. | Standard construction: keep the bounding rectangle and attach each new square to the correct side; use fixed-point scale `r * 1024 / 55`. |
| Low | `precursor-sacred/src/geometry.rs:139-163` | "Sri Yantra" is nine concentric equilateral triangles; they do not interlock. | Figure does not resemble a Sri Yantra. | Label "simplified" in the UI or use a proper vertex table. |
| Low | `precursor-sacred/src/geometry.rs:108-124` | Metatron's Cube uses circle radius r/3 with ring radii r/2 and r. | Circles overlap instead of being tangent (tangent needs radius r/4). | Use r/4. |
| Low | `precursor-sacred/src/ui.rs:125` | Doc says "Bresenham's midpoint algorithm"; body is a 32-segment polygon. | Large circles look faceted. | Fix comment or implement. |
| Low | `precursor-sacred/src/geometry.rs:225-228` | 5 clippy `redundant_field_names` warnings. | - | Tidy. |
| -   | C1-C9 | Common issues. | | |

### Improvements
1. Fix trig, clipping and default sizes; test that polygon vertices lie on the circle within 1 px.
2. Use GAM's `draw_circle(gid, Circle)` instead of 32 line IPCs per circle (Flower of Life is 19 circles = 608 IPCs today).
3. Left/Right to step to the previous/next figure while viewing.

---

## moire ("Moire Generator")

Verdict: nice idea and the XOR-bitmap approach suits a 1-bit panel, but the angle maths is broken so line/grid layers
are solid or diagonal, and the display path is the most IPC-heavy in the slice. **Score 2.5/10.**

### Bugs
| Sev | file:line | Description | Failure scenario | Fix |
|-----|-----------|-------------|------------------|-----|
| Critical | `precursor-moire/src/patterns.rs:82-106,119-141` | Sine bug (X1) via `sin_deg`/`cos_deg`: at angle 0 (and 90/180/270) both are 0, so `proj = 0` for every pixel and the whole layer is set. | Measured: any Lines layer at 0 or 90 deg has 153,216 of 153,216 pixels set. Preset "Zebra" (both layers at 0 deg) XORs to **0 pixels: a blank screen**. "Classic Lines" is an inverted 45-deg pattern rather than two near-parallel gratings. The default menu state (A at 0 deg) has the same problem. | Shared trig (a 0-90 deg table in degrees is enough here). |
| High | `precursor-moire/src/patterns.rs:184-199` | Grid layers at 0 deg are likewise solid (measured: "Fabric Weave" layer A fully set). | "Fabric Weave" and "Grid Rotation" presets show one inverted grid, no weave. | Same. |
| Medium | `precursor-moire/src/patterns.rs:131-135` | Lines duty cycle: a pixel is on only when the band index is even and `within < spacing/2`, so the period is `2*spacing` with 25% duty, and `abs()` mirrors the pattern about the centre giving a double-width band there. Inconsistent with Circles (period `2*spacing`, 50% duty) and Grid (period `spacing`). | "Spacing" means three different things across pattern kinds; a visible seam through the centre. | `((proj.div_euclid(1024)).rem_euclid(spacing)) < spacing/2` for all linear kinds. |
| Medium | `precursor-moire/src/patterns.rs:148-155` | Circles use the octagonal `max + 3/8 min` distance estimate (error up to about 7%). | "Concentric" rings are visibly octagonal; the moire of two such layers shows the octagon artefact, not a true ripple. | Integer sqrt of `dx*dx+dy*dy` (cheap enough at 153K pixels) or compare squared radii incrementally. |
| Medium | `precursor-moire/src/ui.rs:127-159` | Run-length display, one `draw_line` per horizontal run (C8). | Measured runs per frame: Concentric Ripple 15,436; a 45/90-deg line pair 9,826; Fabric 7,790; Radial 4,819. | Bitmap blit. |
| Low | `precursor-moire/src/app.rs:123-127` | Two full layer bitmaps plus the XOR result are live at once (3 x 19,152 B) and reallocated on each render. | Fine for the heap, needless churn. | XOR layer B into A in place; or compute both predicates per pixel and write one bitmap. |
| Low | `precursor-moire/src/patterns.rs:52-54`, `app.rs` | `offset_x/offset_y` exist on `Layer` and drive the best presets but cannot be edited in the menu; `angle` is shown for Circles/Radial where it has no effect. | Users cannot reproduce "Hypnotic"-style patterns. | Add offset fields; hide angle when unused. |
| Low | `precursor-moire/src/app.rs:9,11` | Unused imports (2 warnings). | - | Remove. |
| -   | C1-C9 | Common issues. | | |

Verified fine: `fast_angle_sector` octant mapping is continuous and clamps correctly; bitmap indexing matches the UI;
all per-pixel maths is `i32` (the one `i64` divide is by the constant 8192).

### Improvements
1. Fix trig and unify spacing semantics; add tests (a 0-deg Lines layer must be about 50% set; XOR of identical layers is empty; XOR of spacing 6 vs 7 at 0 deg shows a beat period of 42 px).
2. Live preview: Left/Right in `Preview` nudging layer B's angle by 1 deg is the whole point of a moire toy; today `handle_preview` only exits.
3. Animate (slow rotation) only while focused, with the pump stopped in the background.

---

## lissajous ("Lissajous Machine")

Verdict: correct structure and exact integer phase arithmetic, but wrong sine, a wrong period that overdraws up to
81x, and a table you cannot navigate left in. **Score 2/10.**

### Bugs
| Sev | file:line | Description | Failure scenario | Fix |
|-----|-----------|-------------|------------------|-----|
| Critical | `precursor-lissajous/src/math.rs:29-53` | Sine bug (X1). | The "Circle" preset (1:1, 90 deg) traces a diagonal cross; no preset produces its named shape. | Shared trig. |
| High | `precursor-lissajous/src/app.rs:108-109`, `math.rs:82-85` | Period is taken as `lcm(a,b)` full turns. With `x = sin(a t + d)`, `y = sin(b t)` the curve closes after `1/gcd(a,b)` of a turn (1024/gcd steps), so the code retraces the same curve `lcm*gcd` = `a*b` times, capped at 16,384 points. | Measured: 5:6 computes 16,384 points where 1,024 suffice (16x); 9:9 computes 9,216 where 113 suffice (81x); 2:4 is 8x. Each redundant point is a `(i16,i16)` push and, in `ui.rs:115-125`, a draw IPC. Up to 64 KB of points (more during `Vec` growth) for a 4 KB curve. | `total_steps = 1024 / gcd(a,b)`; reduce a:b by the gcd first; `Vec::with_capacity`. Delete `lcm`. |
| Medium | `precursor-lissajous/src/app.rs:236-253` | In the ratio table `KEY_LEFT` is bound to "back to menu" and there is no way to decrease `table_col`; the footer says "Arrows: select ... Left: back". `table_col` also persists across visits. | After moving right once you can never select a lower column again without restarting the app. | Left moves left; back on Backspace (or Left at column 0). |
| Low | `precursor-lissajous/src/app.rs:256-258` | Table entry always forces phase 90 deg. | For equal odd/even combinations (e.g. 1:1 fine, 2:2, 3:3) fine, but 1:2 at 90 deg degenerates to a parabola arc; the menu phase is silently overwritten. | Keep the user's phase or pick per-ratio defaults. |
| Low | `precursor-lissajous/src/ui.rs:119-120` | Whole-segment rejection (X4); amplitude 140 keeps it on screen today. | Latent. | - |
| Low | `precursor-lissajous/src/app.rs:10,12,21` | Unused imports and unused `KEY_BACKSPACE`. | - | Remove. |
| -   | C1-C9 | Common issues. | | |

Verified fine: phase stepping and wrap (`app.rs:173`), `gcd` (handles zero and negatives), amplitude conversion
(`fp_mul(fp(140), s) >> 16` cannot overflow), `phase_degrees`.

### Improvements
1. Fix trig and period; test closure (`point[0] == point[n]`) and that 1:1 at 90 deg has constant radius within 1 px.
2. Draw thumbnails in the 6x6 ratio table instead of "a:b" text; the header comment says the table shows "pattern shapes" and each curve is only about 100 points at thumbnail size.
3. Animate phase drift (classic oscilloscope look) under a focus-aware pump.
4. Adaptive step: 1024 steps per turn at frequency 9 is 9 units per step; step by `max(1, 4/max(a,b))`-style spacing to keep segments about 2-3 px.

---

## spirograph ("Spirograph Engine")

Verdict: the gcd period logic is right, but it does not compile on its own merits, uses the broken sine, truncates
long-period curves and mis-scales epitrochoids. **Score 2/10.**

### Bugs
| Sev | file:line | Description | Failure scenario | Fix |
|-----|-----------|-------------|------------------|-----|
| Critical | `precursor-spirograph/src/math.rs:45-73` | Sine bug (X1). | No preset matches its name (astroid, deltoid, cardioid all degenerate into diagonal strokes). | Shared trig. |
| High (build) | `precursor-spirograph/src/math.rs:21-24` | `fp_div` is declared `-> i32` but returns an `i64` expression (E0308). This is plain Rust, independent of xous-core. | The crate cannot compile on any toolchain or tree. | `(((a as i64) << FP_SHIFT) / b as i64) as i32`. |
| Medium | `precursor-spirograph/src/app.rs:113-114` | Step count capped at 16,384 = 16 turns, but the curve needs `r/gcd(R,r)` turns (correctly computed by `math.rs:130-134`). | Measured over the reachable menu grid (R 10..200, r 5..R-5, step 5): 179 of 780 combinations need more than 16 turns (worst case 39, e.g. R=200, r=195), so the curve is left open. | Cap by points, not turns: choose the angular step so `period * 1024 / step <= N`, or raise the cap with a coarser step. |
| Medium | `precursor-spirograph/src/app.rs:134-137` | `max_extent = R + d` for both curve types. True extents: hypotrochoid `R - r + d`, epitrochoid `R + r + d`. | Epitrochoids overflow: "Cardioid" (50/50/50) reaches 150/100 x 140 = 210 px from centre on a 168-px half-width, and `ui.rs:122-123` then drops every segment with an off-screen endpoint, leaving gaps. Hypotrochoids are drawn smaller than necessary. | Use the per-type extent. |
| Medium | `precursor-spirograph/src/app.rs:184-189` | `small_r` is clamped against `big_r` only when `small_r` itself is edited; lowering `big_r` afterwards leaves `r >= R`. | R=100, r=40, then reduce R to 30: `diff` goes negative (an "inside-out" hypotrochoid); at `r == R` the curve is a bare circle of radius d. No crash, but the menu shows a state it otherwise forbids. | Re-clamp `small_r` after any `big_r` change. |
| Low | `precursor-spirograph/src/math.rs:26-40` | `SIN_TABLE` is a 1 KB static of zeros that is never read; comments narrate an abandoned approach ("Actually, let's just hardcode the table"). | Dead data in the binary; misleading module doc ("lookup table with linear interpolation"). | Delete or make it the real table (which is the right fix for X1). |
| Low | `precursor-spirograph/src/math.rs:80-94` with `app.rs:123-124` | Doc says "All parameters in fixed-point" but the app passes plain integers; it works only because `fp_div(int,int)` and `fp_mul(int, fp)` happen to compose, and results are truncated to whole units before scaling by 140/extent. | Up to 1 unit of quantisation is magnified to about 1.4-9 px for small R; small curves look jagged. | Pass `fp(R)` etc. and scale in fixed point; shift at the end. |
| Low | `precursor-spirograph/src/math.rs:82-94`, `app.rs:136-137` | Loop-invariant `fp_div` (64-bit divide) is recomputed twice per point, plus two more 64-bit divides for scaling: about 65K soft 64-bit divides for a 16K-point curve. | Tens of ms wasted on RV32; minor next to C8. | Hoist ratio and scale out of the loop. |
| Low | `precursor-spirograph/src/app.rs:9,11,20` | Unused imports/const. | - | Remove. |
| -   | C1-C9 | Common issues. | | |

Verified fine: `curve_period = r / gcd(R, r)` is the correct number of turns for both hypo- and epitrochoids, and
`gcd` is correct; angle accumulation `fp_mul(ratio, t)` has under 0.3 units of error at t = 16,384.

### Improvements
1. Fix `fp_div`, trig, extent and cap; test closure for every preset and that the bounding radius equals the analytic extent within 2 px.
2. Animated drawing (pen progressing over a few seconds) under a focus-aware pump is the natural UX for this app.
3. Share `math.rs` with lissajous: the two files are 65% identical and `ui.rs` 70%.

---

## reaction ("Reaction-Diffusion")

Verdict: the Gray-Scott kernel is correct and numerically stable; the app around it is CPU-bound, never stops its
timer, allocates per step, and three of eight presets do not develop. **Score 3/10.**

### Bugs
| Sev | file:line | Description | Failure scenario | Fix |
|-----|-----------|-------------|------------------|-----|
| High | `precursor-reaction/src/main.rs:50-52,75-79`, `app.rs:139-144` | The 500 ms tick is armed at startup and re-armed forever, regardless of state or focus (C6). In `Running` it keeps stepping and calling `ui::draw` while the app is in the background. | Leave a simulation running, switch apps: the CPU stays pegged and GAM is flooded with draws from a non-focused context until reboot. In `Gallery` it still wakes the SoC twice a second for nothing. | Pump thread that runs only while `Running && auto_running && foreground` (the `apps/ball` pattern). |
| High | `precursor-reaction/src/simulation.rs:89-133`, `app.rs:74-82` | Step cost versus tick: 9,576 cells x (6 `i64` multiplies, 8 non-constant `%`, 10 bounds-checked loads) and two 38 KB `Vec` allocations per step. Estimated 50-100 ms per step at 100 MHz (Suspected; host debug build is 3.2 ms/step). Default 10 steps/frame therefore takes about 0.5-1 s, and the maximum 50 steps/frame about 3-5 s, all inside the message handler. | The app never idles; key presses wait a whole batch (seconds at 50 steps/frame) before being seen; the 500 ms period is fictional. | Time-box the batch (run steps until about 150 ms elapsed), precompute wrap indices or handle borders separately so the inner loop has no `%`, keep two preallocated buffers and `mem::swap`. |
| Medium | `precursor-reaction/src/simulation.rs:178-187` | Preset parameters versus behaviour with these diffusion rates (measured, cells above threshold at steps 10/100/500/1k/2k/4k/6k): "Spots" `[45,72,0,0,0,0,0]` and "Waves" `[49,360,1258,0,0,0,0]` die out to a blank screen; "Fingerprint" stays frozen at the 45-cell seed. Mitosis, Coral, Maze, Holes grow as expected; Worms grows very slowly (288 cells at 6k). | Three of eight gallery entries show nothing or a static square. | Retune with the harness: e.g. spots near f=0.030,k=0.062 need a noisier seed; use `seed_scattered`; verify each preset reaches a non-trivial steady state. |
| Medium | `precursor-reaction/src/simulation.rs:192-193` | `Da = 0.2, Db = 0.1` with a 5-point Laplacian and dt = 1. Stable (needs 4D < 1), and the 2:1 ratio is right, but features are only a few cells wide on an 84x114 grid and growth is slow: Mitosis needs about 2,000 steps to fill a quarter of the grid, which at 10 steps per 0.5-1 s is several minutes. | "Live simulation" looks frozen for the first minute. | Default to more steps per frame once the step is optimised; or seed with several blobs. |
| Medium | `precursor-reaction/src/simulation.rs:95-96,131-132` | Heap: `a`, `b`, `na`, `nb` are each 9,576 x 4 B = 38.3 KB, so 153 KB live during a step plus the 19 KB display bitmap, with 76.6 KB allocated and freed every step (766 KB of churn per default frame). | Not an OOM on its own, but it is the largest heap user in the slice and fragments needlessly. | Two persistent buffers; `u16` 0.16 fixed point halves it (values are clamped to [0,1] anyway). |
| Low | `precursor-reaction/src/simulation.rs:69-86` | `seed_scattered` is dead code, so no randomness is ever used (and no TRNG dependency exists). | Every run of a preset is identical. | Wire it to TRNG and a "reseed" key. |
| Low | `precursor-reaction/src/app.rs:129-132` | Right arrow runs a batch even when not paused (README says "when paused"). | Harmless. | Gate on `!auto_running`. |
| Low | `precursor-reaction/src/app.rs:81`, `simulation.rs:146` | Fixed threshold B > 0.25 while steady-state B peaks at only 0.32-0.43 (measured). | Thin margins: thresholded shapes flicker at edges. | Threshold near 0.2 or ordered dither of B for a far better 1-bit rendering. |
| Low | `precursor-reaction/src/app.rs:9`, `simulation.rs:45` | Unused import; unnecessary `mut`. | - | Remove. |
| -   | C1-C9 | Common issues. | | |

Verified fine: update equations and clamping (`simulation.rs:118-127`), toroidal Laplacian indices, no overflow
(`4 * FP_ONE` and all products fit), 4x upscale bitmap geometry (336x456), `seed_center` bounds.

### Improvements
1. Focus-aware, time-boxed stepping; double buffer; no `%` in the inner loop.
2. Retune or replace the three dead presets; add a harness test asserting each preset has between 5% and 80% coverage after N steps.
3. Dithered output and a bitmap blit (C8): the 4x4 block run-length path sends each run four times (once per scanline of the block).
4. Persist nothing, but show elapsed steps/s in the header so the user knows it is alive.

---

## life ("Conway's Life")

Verdict: the simulation core is correct (verified against known patterns); the shell around it has the common key,
timer and API problems plus an unreachable menu entry. **Score 4/10.**

### Bugs
| Sev | file:line | Description | Failure scenario | Fix |
|-----|-----------|-------------|------------------|-----|
| High | `precursor-life/src/main.rs:25,38-41`, `app.rs:45-50` | Tick re-arms unconditionally at 50-1000 ms in every state and regardless of focus; in `Running` it steps and redraws in the background (C6). | Battery drain at up to 20 wakeups/s for as long as the device is on, even sitting in the pattern list or with the app backgrounded. | Focus-aware pump that only runs while `Running && !paused && foreground`. |
| Medium | `precursor-life/src/app.rs:65` with `ui.rs:46,57-63` | The list draws `PATTERNS.len() + 2` entries ("Random", "Empty (Editor)") but the cursor stops at `PATTERNS.len()`. | "Empty (Editor)" is displayed and can never be selected; the `else` branch at `app.rs:75` is unreachable. | `pattern_cursor < PATTERNS.len() + 1`, and route the last entry to the editor. |
| Medium | `precursor-life/src/app.rs:92` with C2 | The only way from `Running` back to the pattern list is `KEY_MENU`, which GAM swallows. | Once a pattern is started there is no way to pick another one (only `R`, `E`, Space work). | Add a deliverable back key. |
| Medium | `precursor-life/src/grid.rs:59-88` | Per generation: 9,576 cells x 8 neighbours x 2 non-constant `%` plus a fresh 9.5 KB `Vec<bool>`; estimated 60-100 ms at 100 MHz (Suspected), then about 1,800 rectangle IPCs for a 25% random field (C8). | The 50 ms speed setting is unattainable and the handler blocks input for a frame. | Bit-packed rows (84 bits = 3 `u32`), wrap handled by padding columns, two persistent buffers; draw via bitmap blit or only changed cells. |
| Low | `precursor-life/src/app.rs:100-106` | `R` (reset) clears the grid and re-stamps only if a pattern is selected. | After "Random", `R` leaves an empty grid with the sim still "running". | Re-randomise in that case. |
| Low | `precursor-life/src/main.rs:23` | PRNG seed is `elapsed_ms() as u32`, not the TRNG the device has. | Predictable/low-entropy seeds shortly after boot; cosmetic. | `trng.get_u32()`. |
| Low | `precursor-life/src/app.rs:120` | Leaving the editor with Menu force-unpauses. | Edits start evolving immediately even if the user paused before entering. | Preserve prior `paused`. |
| Low | `precursor-life/src/ui.rs:106` | Footer text is longer than the 320-px text box in the Small style. | Truncated help line (Suspected; depends on glyph widths). | Shorten or two lines. |
| -   | C1-C9 | Common issues. | | |

Verified (measured): toroidal wrapping is correct in both axes (glider population stays 5 across 800 generations
and multiple wraps); all 10 pattern tables have `len == width*height`; blinker, toad, beacon, pulsar (48), LWSS
(9/12 alternation), Gosper gun (36 -> 41 -> 46 at 30-generation intervals) behave canonically. B3/S23 logic at
`grid.rs:80` is right. `Vec<bool>` at 9.5 KB is no heap concern.

### Improvements
1. Merge with `death`: Life is rule 0 of the Death engine; `grid.rs` and `automaton.rs` are the same file with a rule table added (see cross-cutting).
2. On a torus the Gosper gun shoots itself after about 300 generations; offer a "dead border" option.
3. Draw only cell deltas between generations, or a single bitmap blit.
4. 2-px cells (168x228) as hinted in `grid.rs:7` become feasible once the grid is bit-packed.

---

## death ("Game of Death")

Verdict: same engine as life with a correct 12-rule table; same shell problems. **Score 4/10.**

### Bugs
| Sev | file:line | Description | Failure scenario | Fix |
|-----|-----------|-------------|------------------|-----|
| High | `precursor-death/src/main.rs:25,38-41`, `app.rs:54-60` | Unconditional tick, background stepping and drawing (as life). | Same battery/CPU drain. | Focus-aware pump. |
| Medium | `precursor-death/src/app.rs:106` with C2 | `Running` -> `RuleSelect` only via `KEY_MENU`. | Cannot change rule after the first start. | Deliverable back key. |
| Medium | `precursor-death/src/automaton.rs:95-124` | Same per-generation cost and allocation pattern as life. | Speed settings below about 100 ms are fictional. | Shared bit-packed engine. |
| Medium | `precursor-death/src/app.rs:86-92,111-115` | Every rule starts from the same 25% random soup. | Rules designed around other densities give a poor first impression: Anneal and Diamoeba collapse quickly from 25%, Day & Night is symmetric around 50%, Seeds/Replicator explode from anything but a tiny seed. | Per-rule start density/pattern in the preset table. |
| Low | `precursor-death/src/automaton.rs:31-37` | `rule()` silently drops digits >= 9; there is no rule-string parser at all despite "arbitrary Birth/Survival rules" in the module doc. | Typos in presets vanish silently; users cannot enter a custom `Bx/Sy`. | `const` rule masks (`u16` bitmasks) checked at compile time; add a small B/S editor (two rows of 9 toggles). |
| Low | `precursor-death/src/app.rs:56,108` | `Rule` is cloned every tick to satisfy the borrow checker. | Negligible cost; noise. | `#[derive(Clone, Copy)]` or split borrows; with `u16` masks it is two words. |
| Low | `precursor-death/src/automaton.rs:82-93` | `get/set/toggle` lost the bounds checks that life's copy has. | Safe with current callers (all indices derive from grid dims). | Keep one implementation. |
| Low | `precursor-death/src/ui.rs:78-80` | Header such as `PAUSED B3678/S235678 G:12345 P:1234` in Bold exceeds the 320-px box. | Truncated header (Suspected). | Put the rule name in the footer. |
| Low | `precursor-death/src/main.rs:23` | Uptime-seeded PRNG. | Cosmetic. | TRNG. |
| -   | C1-C9 | Common issues. | | |

Verified (measured): all 12 rule tables round-trip to their canonical B/S notation (HighLife B36/S23, Day & Night
B3678/S34678, Seeds B2/S, Diamoeba B35678/S5678, Morley B368/S245, Anneal B4678/S35678, 2x2 B36/S125, Maze
B3/S12345, Coagulations B378/S235678, Replicator B1357/S1357, Stains B3678/S235678). Neighbour wrapping is the same
code as life. The neighbour count (max 8) indexes `[bool; 9]` safely. Scroll window logic at `app.rs:74-85` is right.

### Improvements
1. Fold life's pattern library into this app and ship one "Cellular" app with rule + pattern selection.
2. Custom rule editor; show the rule name while running.
3. Per-rule seeds; generation-rate readout.

---

## Cross-cutting

### X1. One wrong sine, eight copies (Critical)

Locations: `precursor-penrose/src/tiling.rs:80-93`, `precursor-kaleidoscope/src/mirror.rs:19-32`,
`precursor-sacred/src/geometry.rs:17-31`, `precursor-sacred/src/ui.rs:149-175` (twice inline),
`precursor-moire/src/patterns.rs:82-102`, `precursor-lissajous/src/math.rs:29-48`,
`precursor-spirograph/src/math.rs:45-68`.

The code folds the angle to a quarter wave `idx in 0..=256` and then evaluates `4*idx*(256-idx)/256^2`. That parabola
is the classic approximation for a *half* wave; over a quarter wave it peaks at idx=128 and returns to 0 at idx=256.
Measured against the true function (1024 units per turn):

| angle | code sin | true sin | code cos | true cos |
|------:|---------:|---------:|---------:|---------:|
| 0 | 0.000 | 0.000 | 0.000 | 1.000 |
| 64 (22.5 deg) | 0.750 | 0.383 | 0.750 | 0.924 |
| 128 (45 deg) | 1.000 | 0.707 | 1.000 | 0.707 |
| 256 (90 deg) | 0.000 | 1.000 | 0.000 | 0.000 |
| 512 (180 deg) | 0.000 | 0.000 | 0.000 | -1.000 |

`|cos| == |sin|` at every angle, so `(r cos, r sin)` only ever lands on the two diagonals: the "unit circle" has
radius ranging 0.000..1.414 (measured). Minimal fix is `idx*(512-idx)/256^2` (half-wave parabola, about 5.6% peak
error, still visibly non-round); the right fix is a 257-entry quarter-wave table (514 bytes as `u16`) with linear
interpolation, in one place. A single host test (`sin(256) == ONE`, `sin^2+cos^2` within 1%) would have caught this
in all six apps.

### X2. Input model is wrong everywhere (C1, C2, C3)
PUA arrow codes, a Menu key the app cannot receive, and process termination on "quit". All ten apps share the same
seven-line constant block and the same exit code, so the fix is one shared `Key` decoder and one run loop.

### X3. Timers and focus (C6)
No app decodes `FocusState`. The three animated apps re-arm a timer unconditionally through an API that does not
exist. On the real system the equivalent is a pump thread; it must be told to stop on `Background` and when the app's
own state is idle (gallery, paused, editor). The seven static apps are fine in this respect: they are purely
event-driven and block in `receive_message`.

On display rate: the Precursor panel is a 1-bit memory LCD rather than e-ink, so 5-10 fps is physically fine; the
constraint is CPU and IPC cost per frame (C8), and battery when nothing gates the pump.

### X4. Clipping by rejection
`if both endpoints inside { draw }` appears in penrose, kaleidoscope, sacred (twice), lissajous and spirograph.
It silently deletes geometry at the edges (sacred and spirograph are visibly affected). GAM clips to the canvas
already; if the header/footer must be protected, use a real Cohen-Sutherland clip in one shared function, or draw the
header/footer last over a cleared band.

### X5. Synchronous heavy work in the key handler
flames (up to 200K iterations), penrose (level 7), moire (2 x 153K pixels), spirograph/lissajous (16K points),
reaction (up to 50 steps) all compute inside `handle_key`/`tick` on the only thread. Nothing shows a "rendering..."
state first. A shared "compute in slices, then draw" helper solves this once.

### X6. Float / performance summary
- No `f32`/`f64` anywhere: good, and worth keeping (soft-float on VexRiscv is expensive).
- `i64` multiplies in `fp_mul` are acceptable (two `mul`/`mulhu`); `i64` divides (`fp_div`, scaling in spirograph
  `app.rs:136-137`) are slow library calls and are all loop-invariant or avoidable.
- Non-constant `%` for wrapping in reaction/life/death inner loops is the main CPU cost there (dims are `const`s but
  are copied into struct fields, which defeats constant folding). Use the constants directly or remove the modulo.
- Per-step heap allocation in reaction (76.6 KB), life and death (9.5 KB): replace with double buffering.
- Peak heap, largest first: reaction about 172 KB; lissajous/spirograph about 64-128 KB during `Vec` growth
  (no `with_capacity`); moire 57 KB; flames 19 KB; penrose small today, about 200 KB+ at level 7 once the
  subdivision is correct. None is dangerous alone, but all are avoidable.

### X7. Duplication, quantified

Method: normalised non-blank, non-comment lines of 12+ characters, compared across the ten `src/` trees.

| App | Code lines | Lines also present in another app | Lines present in 5+ apps |
|-----|-----------:|----------------------------------:|-------------------------:|
| flames | 404 | 171 (42%) | 122 (30%) |
| penrose | 448 | 156 (35%) | 133 (30%) |
| kaleidoscope | 480 | 209 (44%) | 162 (34%) |
| sacred | 530 | 149 (28%) | 124 (23%) |
| moire | 642 | 209 (33%) | 152 (24%) |
| lissajous | 562 | 256 (46%) | 180 (32%) |
| spirograph | 507 | 233 (46%) | 159 (31%) |
| reaction | 451 | 170 (38%) | 127 (28%) |
| life | 447 | 218 (49%) | 66 (15%) |
| death | 397 | 204 (51%) | 67 (17%) |
| **Total** | **4,868** | **1,975 (41%)** | |

Specific clones:
- `main.rs`: eight files are 87-92% line-identical to flames' (the diff is the module name, two string constants and
  0-5 lines of TRNG/tick wiring). life and death are the same loop minified: 88% token-identical to flames, and 99%
  identical to each other (4 differing lines).
- `ui.rs` `draw_header`/`draw_footer`/`draw` and the `SCREEN_W/HEADER_H/FOOTER_Y` block: identical in all ten.
- Key constant block: identical in all ten (and wrong in all ten).
- Menu-field renderer (highlight rectangle + inverted `TextView` per row) and gallery list with scroll window: seven
  near-identical copies.
- Sine: eight copies (X1). LCG `x*1103515245+12345`: 13 occurrences in 7 files.
- Bitmap run-length renderer: three copies (flames `ui.rs:100-134`, moire `ui.rs:127-159`, reaction `ui.rs:105-135`);
  flames vs reaction `ui.rs` are 79% identical.
- Cell-run renderer: three copies (life `ui.rs` twice, death once).
- lissajous vs spirograph: `math.rs` 65%, `ui.rs` 70%, `app.rs` 55% identical.
- life vs death: `app.rs` 72%, grid/automaton 55%, `ui.rs` 54% identical; functionally death is a superset of life
  minus the pattern table.

Realistically about 1,900-2,200 of the 4,868 lines disappear with a shared crate, and bugs X1, C1-C8 and X4 each
get fixed once instead of 6-10 times.

### X8. Proposed shared crate: `precursor-genkit`

Layout (one repo, vendored into `xous-core/libs/genkit`, apps depend on `genkit = { path = "../../libs/genkit" }`).
Two layers so the maths stays host-testable:

```
genkit/
  src/lib.rs
  src/fixed.rs     # pure, no_std
  src/rng.rs       # pure
  src/bitmap.rs    # pure
  src/cagrid.rs    # pure
  src/geom.rs      # pure
  src/keys.rs      # pure
  src/shell.rs     # feature "xous": run loop, focus, pump
  src/canvas.rs    # feature "xous": GAM wrapper
  src/widgets.rs   # feature "xous": header/footer/menu/list
```

Pure layer (default features, `cargo test` on host):

```rust
// fixed.rs
#[derive(Copy, Clone, PartialEq, PartialOrd)] pub struct Fx(pub i32);      // 16.16
impl Fx { pub const ONE: Fx; pub const fn from_int(i32)->Fx; pub const fn milli(i32)->Fx;
          pub fn mul(self, Fx)->Fx; pub fn div(self, Fx)->Option<Fx>; pub fn round(self)->i32; }
pub type Angle = i32;                         // 1024 units per turn (or 4096; one constant)
pub fn sin(a: Angle) -> Fx;                   // 257-entry quarter-wave table + lerp
pub fn cos(a: Angle) -> Fx;
pub fn sin_cos(a: Angle) -> (Fx, Fx);
pub fn deg(d: i32) -> Angle;
pub fn atan2(dy: i32, dx: i32) -> Angle;
pub fn isqrt(n: u32) -> u32;
pub const fn gcd(a: u32, b: u32) -> u32;

// rng.rs
pub struct Lcg(u32);                          // or xorshift32
impl Lcg { pub fn new(seed: u32)->Self; pub fn next_u32(&mut self)->u32;
           pub fn below(&mut self, n: u32)->u32; pub fn chance(&mut self, num: u32, den: u32)->bool; }

// bitmap.rs
pub struct Bitmap1 { w: u16, h: u16, bits: Vec<u8> }     // row-major, MSB first
impl Bitmap1 { pub fn new(w,h)->Self; pub fn clear(&mut self); pub fn set(&mut self,x,y);
               pub fn get(&self,x,y)->bool; pub fn xor_with(&mut self,&Bitmap1);
               pub fn fill_block(&mut self,x,y,size);          // 4x upscale for reaction/CA
               pub fn row_runs(&self,y)->impl Iterator<Item=(u16,u16)>; pub fn popcount(&self)->u32; }

// geom.rs
pub struct Seg { pub x0:i16, pub y0:i16, pub x1:i16, pub y1:i16 }
pub struct ClipRect { .. }
pub fn clip(seg: Seg, r: &ClipRect) -> Option<Seg>;            // Cohen-Sutherland
pub fn polar(cx:i16, cy:i16, r:i32, a:Angle) -> (i16,i16);
pub fn rotate(p:(i16,i16), c:(i16,i16), a:Angle) -> (i16,i16);
pub fn circle_points(cx,cy,r, f: impl FnMut(i16,i16));          // midpoint

// cagrid.rs  (replaces life/grid.rs and death/automaton.rs)
#[derive(Copy, Clone)] pub struct Rule { pub birth: u16, pub survive: u16 }   // bit n = n neighbours
impl Rule { pub const LIFE: Rule; pub const fn parse(s:&str)->Option<Rule>; pub fn notation(&self)->String; }
pub struct CaGrid { .. }                                         // bit-packed, double-buffered, toroidal
impl CaGrid { pub fn new(w,h)->Self; pub fn step(&mut self, r:Rule); pub fn get/set/toggle(..);
              pub fn stamp(&mut self, pat:&Pattern, cx, cy); pub fn randomize(&mut self,&mut Lcg, pct:u8);
              pub fn population(&self)->u32; pub fn generation(&self)->u32;
              pub fn to_bitmap(&self, cell_px:u8, out:&mut Bitmap1); }

// keys.rs
pub enum Key { Up, Down, Left, Right, Enter, Back, Space, Char(char) }
impl Key { pub fn from_char(c: char) -> Option<Key>; }           // U+2190..2193, 0x0D, 0x08, ...
```

Xous layer (`features = ["xous"]`):

```rust
// shell.rs
pub enum Flow { Continue, Redraw, Background }                   // Background = relinquish focus, stay alive
pub trait GenApp {
    const SERVER_NAME: &'static str;
    const APP_NAME: &'static str;                                // == gam::APP_NAME_* / manifest context_name
    fn new(seed: u32) -> Self;
    fn on_key(&mut self, key: Key) -> Flow;
    fn tick_ms(&self) -> Option<u32> { None }                    // None = no pump; re-queried after each event
    fn on_tick(&mut self) -> Flow { Flow::Continue }             // only called while foreground
    fn draw(&self, c: &mut Canvas);
}
pub fn run<A: GenApp>() -> !;       // registers Framebuffer UX, gets canvas, decodes rawkeys,
                                    // tracks FocusState, owns the pump thread, never terminates

// canvas.rs
pub struct Canvas { gam: Gam, gid: Gid, bounds: Point, batch: GamObjectList }
impl Canvas { pub fn size(&self)->(i16,i16); pub fn content(&self)->ClipRect;   // minus header/footer
              pub fn clear(&mut self); pub fn line(&mut self, s: Seg);          // clipped + batched (32/IPC)
              pub fn rect_fill(&mut self, ..); pub fn circle(&mut self, cx,cy,r);
              pub fn blit(&mut self, x:i16, y:i16, bm:&Bitmap1);                // tiles; falls back to runs
              pub fn text(&mut self, rect, style: GlyphStyle, invert: bool, s:&str);
              pub fn flush(&mut self); }                                         // flush batch + gam.redraw()

// widgets.rs
pub fn header(c:&mut Canvas, title:&str);  pub fn footer(c:&mut Canvas, hint:&str);
pub struct ListMenu { pub cursor: usize, pub scroll: usize, pub visible: usize }
impl ListMenu { pub fn on_key(&mut self, k:Key, len:usize)->bool;
                pub fn draw(&self, c:&mut Canvas, len:usize, label: impl Fn(usize)->String); }
pub struct FieldMenu<F: Copy + PartialEq> { .. }                 // "< value >" rows + buttons
```

Each app's `main.rs` becomes `fn main() -> ! { genkit::shell::run::<FlamesApp>() }` and keeps only its algorithm
module and a short `impl GenApp`. Suggested consolidation while migrating: life + death -> one app;
lissajous + spirograph -> one "Curves" app sharing the point-list renderer; flames, moire and reaction share
`Bitmap1` + `Canvas::blit`.

Tests that should ship with the crate (all would have failed on today's code): `sin(90deg) == 1`,
`sin^2 + cos^2` within 1%; `Key::from_char('\u{2191}') == Up`; `clip()` properties; `CaGrid` glider survives a wrap;
`Rule::parse("B36/S23")` round-trips; `Bitmap1::row_runs` round-trips set bits.

### X9. Packaging and docs
- READMEs describe controls and architecture but not installation (manifest entry, `cargo xtask app-image` /
  `renode-image` invocation, context name). `AGENTS.md`/`CLAUDE.md` in each repo assert properties that are not true
  of the code ("testable without Xous" with zero tests; life's Empty entry; spirograph's "lookup table").
- Two different dependency pin sets across ten sibling apps; unused deps (`xous-ipc`, `ticktimer-server` in the seven
  static apps).
- No CI. A single GitHub Action running `cargo test` on the pure layer and `cargo check` of each app inside a pinned
  xous-core checkout would have caught every Critical item in this report except C1/C2.

### Suggested order of work
1. Stand up `genkit` with `fixed`, `keys`, `shell`, `canvas` and tests; port life first (its logic is already correct), which proves the shell on Renode.
2. Port death (merge with life), then reaction (bitmap blit + focus-aware pump).
3. Port lissajous/spirograph on the fixed trig and correct periods.
4. Port moire and flames (viewport fix).
5. Rewrite penrose subdivision; fix sacred constructions; kaleidoscope wedge maths.
