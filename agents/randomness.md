# Randomness Agent

## Role
Implement true random number generation patterns for Precursor apps using the hardware TRNG. Ensure uniform distributions, proper bias elimination, and correct randomness algorithms.

## Expertise
- Hardware TRNG access via `trng::Trng` crate
- Rejection sampling for uniform distribution (modulo bias elimination)
- Fisher-Yates shuffle for permutation generation
- Weighted random selection from non-uniform distributions
- Dice notation parsing and evaluation
- Card deck management (draw without replacement)
- Random selection patterns (pick, weighted pick)

## Key Patterns

Prefer `precursor_appkit::rng` (fail-closed TRNG with unbiased range). The raw patterns follow; all
are compiled in `tests/doc-snippets/src/rng.rs`.

### TRNG Wrapper (fail closed)
A TRNG error must never turn into a constant. `unwrap_or(0)` would make `range()` return 0 forever
(0 is always under the rejection threshold), silently making every roll, shuffle, key or pad
deterministic. Panic (or propagate the error) instead.
```rust
pub struct Rng {
    trng: trng::Trng,
}

impl Rng {
    pub fn new(xns: &xous_names::XousNames) -> Self {
        Self { trng: trng::Trng::new(xns).expect("can't connect to TRNG") }
    }

    /// Fails closed: a TRNG error must never silently become a constant.
    pub fn u32(&self) -> u32 {
        self.trng.get_u32().expect("TRNG read failed")
    }

    pub fn flip(&self) -> bool {
        self.u32() & 1 == 1
    }
}
```

### Rejection Sampling (REQUIRED for range())
Never use raw modulo — it introduces bias when `max` doesn't evenly divide 2^32.
```rust
pub fn range(&self, max: u32) -> u32 {
    if max <= 1 { return 0; }
    let threshold = u32::MAX - (u32::MAX % max); // a multiple of max
    loop {
        let val = self.u32();
        if val < threshold {
            return val % max;
        }
    }
}
```

### Fisher-Yates Shuffle
Standard algorithm — iterate in reverse, swap with random index in remaining range.
```rust
pub fn shuffle<T>(&self, items: &mut [T]) {
    let n = items.len();
    for i in (1..n).rev() {
        let j = self.range((i + 1) as u32) as usize;
        items.swap(i, j);
    }
}
```

### Weighted Selection
Cumulative weight scan — works with any non-negative weights whose sum fits in `u32`
(a plain `.sum()` would panic on overflow in debug and wrap in release).
```rust
pub fn weighted_pick(&self, weights: &[u32]) -> usize {
    let total: u32 = weights
        .iter()
        .try_fold(0u32, |acc, &w| acc.checked_add(w))
        .expect("weights sum overflows u32");
    if total == 0 { return 0; }
    let mut roll = self.range(total);
    for (i, &w) in weights.iter().enumerate() {
        if roll < w { return i; }
        roll -= w;
    }
    weights.len() - 1
}
```

## Quality Criteria
- TRNG errors fail closed (panic or propagate) — never `unwrap_or(0)` or any default value
- Rng wrapper has NO GAM/PDDB dependencies (extractable as library)
- Rejection sampling MUST be used for `range()` — never raw modulo
- Fisher-Yates iterates in reverse (standard algorithm)
- Weighted pick handles zero-total gracefully (returns 0)
- All random operations are const-time per sample (except rejection loop)
- `flip()` uses LSB of `u32()` for fastest boolean generation

## Cargo Dependency
```toml
trng = { path = "../../services/trng" }
```

## Handoffs
- FROM architecture.md: "App needs randomness"
- TO any module: "Use `&Rng` reference for all random decisions"
- Rng created once in `main()`, passed by reference throughout

## Born From
- App #3: Decision Engine (precursor-decide)
- Validated patterns: dice rolling, card shuffling, coin flipping, weighted spinners, tournament seeding
