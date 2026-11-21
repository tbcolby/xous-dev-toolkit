# Encoding Agent

**Role**: Implement data encoding algorithms (QR codes, barcodes, checksums, error correction) optimized for Precursor's 1-bit display.

**Born from**: App #1 (QR Code Generator)

---

## Expertise

### Finite Field Arithmetic
- Galois field GF(2^8) with configurable primitive polynomials
- Precomputed exp/log tables for O(1) multiplication
- Polynomial multiplication, division, remainder over GF(2^8)
- Primitive polynomial for QR: 0x11D (x^8 + x^4 + x^3 + x^2 + 1)

### Error Correction
- Reed-Solomon encoding (systematic form: data + EC codewords)
- Generator polynomial construction via iterative root multiplication
- BCH codes for format/version information
- Block interleaving for multi-block RS structures

### Data Encoding Pipelines
- Bit stream construction with variable-width writes
- Mode indicators and character count fields
- Numeric encoding (3 digits → 10 bits, 2 → 7, 1 → 4)
- Alphanumeric encoding (2 chars → 11 bits, 1 → 6)
- Byte encoding (8 bits per character)
- Terminator insertion, byte alignment, pad codeword alternation (0xEC/0x11)

### Matrix/Grid Construction
- Module placement with reserved region tracking
- Finder patterns (7x7 with separators)
- Alignment patterns (5x5, position tables per version)
- Timing patterns (alternating modules)
- Format information placement (dual-location for redundancy)
- Data placement via zigzag traversal, skipping reserved modules

### Mask Evaluation
- Eight QR mask conditions
- Four penalty rules (consecutive runs, 2x2 blocks, finder-like, proportion)
- Full evaluation: clone matrix × 8 masks × score → select minimum
- Format info encoding per selected mask

### Barcode Standards (for App #2+)
- Code 128 (alphanumeric, full ASCII)
- Code 39 (uppercase + digits)
- EAN/UPC (product codes)
- DataMatrix (2D, alternative to QR)
- Checksum computation per standard

---

## Patterns

### Encoder Module Structure
```text
src/
├── encode.rs       # Public API: encode(text, options) -> Matrix
├── gf.rs           # Galois field arithmetic (optional split)
├── rs.rs           # Reed-Solomon (optional split)
├── modes.rs        # Data encoding modes
├── matrix.rs       # Grid construction
├── mask.rs         # Mask patterns and penalty
└── tables.rs       # Version/capacity/alignment tables
```

For small encoders (QR, barcodes), a single file is acceptable.

### Key Design Principles

1. **Zero Xous dependencies** in encoding logic. The encoder must be extractable as a standalone `no_std` library.

2. **Auto-detection**: Always detect the optimal encoding mode for the input data. Don't force the user to choose.

3. **Auto-sizing**: Always select the minimum version/size that fits the data. Waste no space.

4. **Precomputed tables**: For field arithmetic, precompute at init time, not compile time. This keeps binary size small while enabling O(1) operations.

5. **Clone-and-evaluate**: For mask selection or any optimization with multiple candidates, clone the state, try each, measure, pick best.

---

## Rendering Patterns

Compiled in `tests/doc-snippets/src/encoding.rs`. Uses only GAM rectangles, batched with
`GamObjectList` (flush when `push` returns `Err` because the list is full).

### Auto-Fit Calculation
Return `None` instead of clamping up: a floor of 2 px with 336 px of width only fits 168 total
modules (QR version 35 with a 4-module quiet zone); clamping larger codes up to 2 px makes
`display_width - total_px` underflow (panic in debug, garbage offset in release).
```rust
/// Largest module size (px) that fits, capped at 8. None if the code can't fit at >= 1 px.
fn auto_fit_size(modules: usize, display_w: usize, display_h: usize, quiet: usize) -> Option<usize> {
    let total_modules = modules + quiet * 2;
    let px = (display_w.min(display_h) / total_modules).min(8);
    if px == 0 { None } else { Some(px) }
}
```
Modules below 2 px are hard for phone cameras to read on the memory LCD; prefer capping the QR version.

### Module-to-Pixel Mapping
```rust
use gam::menu::*;
use gam::{Gam, GamObjectList, GamObjectType};

fn push_rect(gam: &Gam, gid: Gid, list: &mut GamObjectList, r: Rectangle) -> Result<(), xous::Error> {
    if let Err(obj) = list.push(GamObjectType::Rect(r)) {
        // list full: flush and start a new batch
        let full = std::mem::replace(list, GamObjectList::new(gid));
        gam.draw_list(full)?;
        list.push(obj).ok();
    }
    Ok(())
}

/// `modules[row][col] == true` means a dark module. Square matrix.
fn draw_matrix(gam: &Gam, gid: Gid, canvas: Point, modules: &[Vec<bool>], quiet: usize) -> Result<(), xous::Error> {
    let size = modules.len();
    let (w, h) = (canvas.x as usize, canvas.y as usize);
    let Some(px) = auto_fit_size(size, w, h, quiet) else {
        log::warn!("code too large for the display");
        return Ok(());
    };
    let total_px = (size + quiet * 2) * px; // <= min(w, h), so the subtractions can't underflow
    let x0 = ((w - total_px) / 2 + quiet * px) as isize;
    let y0 = ((h - total_px) / 2 + quiet * px) as isize;
    let px = px as isize;
    let dark = DrawStyle::new(PixelColor::Dark, PixelColor::Dark, 0);

    let mut list = GamObjectList::new(gid);
    for (row, line) in modules.iter().enumerate() {
        for (col, &is_dark) in line.iter().enumerate() {
            if is_dark {
                let (x, y) = (x0 + col as isize * px, y0 + row as isize * px);
                push_rect(gam, gid, &mut list, Rectangle::new_coords_with_style(x, y, x + px - 1, y + px - 1, dark))?;
            }
        }
    }
    gam.draw_list(list)?;
    gam.redraw()
}
```

### 1-Bit Barcode Rendering
For 1D barcodes, modules are vertical bars spanning a configurable height:
```rust
fn draw_barcode(gam: &Gam, gid: Gid, bars: &[bool], x_start: isize, y_start: isize, bar_width: isize, height: isize)
    -> Result<(), xous::Error> {
    let dark = DrawStyle::new(PixelColor::Dark, PixelColor::Dark, 0);
    let mut list = GamObjectList::new(gid);
    for (i, &is_dark) in bars.iter().enumerate() {
        if is_dark {
            let x = x_start + i as isize * bar_width;
            push_rect(gam, gid, &mut list, Rectangle::new_coords_with_style(x, y_start, x + bar_width - 1, y_start + height, dark))?;
        }
    }
    gam.draw_list(list)
}
```

---

## Quality Criteria

- [ ] Encoder has zero Xous dependencies
- [ ] All encoding tables are compile-time or deterministic runtime
- [ ] Auto-mode detection (optimal encoding for input)
- [ ] Auto-version/size selection (minimum for data)
- [ ] Rendering uses only GAM rectangle primitives
- [ ] Quiet zone enforced per specification
- [ ] Error correction level configurable
- [ ] Output scannable by standard readers (phones, dedicated scanners)

---

## Handoffs

| Direction | Agent | What |
|-----------|-------|------|
| FROM | ideation.md | Encoding standard to implement |
| FROM | architecture.md | Module decomposition, single file vs split |
| TO | graphics.md | Matrix rendering on display |
| TO | storage.md | Persisting encoded data or templates |
| TO | build.md | Library crate extraction decision |
| TO | testing.md | Validation via scanner apps / visual inspection |

---

## Reference Implementation

See `precursor-qrcode/src/qr_encode.rs` — the first encoding module built for the ecosystem. ~600 lines, complete QR encoder, zero dependencies.
