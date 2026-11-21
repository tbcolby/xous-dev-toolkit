//! Fallible randomness and unbiased selection.
//!
//! Every generator takes an [`Entropy`] source whose draws can fail. A failed
//! draw aborts the whole generation with [`EntropyError`]; nothing partially
//! random is ever returned. On the device the source is the hardware TRNG
//! (`precursor_appkit::rng::Trng` implements [`Entropy`]); in tests use the
//! sources in [`testing`] (enable this crate's `testing` feature in
//! `[dev-dependencies]`).

/// The entropy source failed or looks stuck. Nothing was generated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntropyError;

/// A source of uniformly random `u32`s that can fail.
pub trait Entropy {
    fn try_u32(&mut self) -> Result<u32, EntropyError>;
}

/// Rejection loops give up after this many consecutive rejections. For a
/// working source the chance of reaching it is below 2^-64; a source that
/// returns a constant reaches it immediately and is reported as broken
/// instead of spinning forever.
pub const MAX_REJECTIONS: u32 = 64;

/// Uniform value in `0..n` without modulo bias (Lemire's multiply-shift with
/// rejection). `n` must be non-zero.
pub fn below<E: Entropy + ?Sized>(e: &mut E, n: u32) -> Result<u32, EntropyError> {
    assert!(n > 0, "below: n must be > 0");
    let threshold = n.wrapping_neg() % n;
    for _ in 0..=MAX_REJECTIONS {
        let m = (e.try_u32()? as u64) * (n as u64);
        if (m as u32) >= threshold {
            return Ok((m >> 32) as u32);
        }
    }
    Err(EntropyError)
}

/// Uniformly chosen element of `items` (non-empty).
pub fn pick<'a, T, E: Entropy + ?Sized>(e: &mut E, items: &'a [T]) -> Result<&'a T, EntropyError> {
    Ok(&items[below(e, items.len() as u32)? as usize])
}

/// Fisher–Yates shuffle. On error `items` may be partially shuffled.
pub fn shuffle<T, E: Entropy + ?Sized>(e: &mut E, items: &mut [T]) -> Result<(), EntropyError> {
    for i in (1..items.len()).rev() {
        let j = below(e, (i + 1) as u32)? as usize;
        items.swap(i, j);
    }
    Ok(())
}

/// Fill `dest` with random bytes.
pub fn fill<E: Entropy + ?Sized>(e: &mut E, dest: &mut [u8]) -> Result<(), EntropyError> {
    for chunk in dest.chunks_mut(4) {
        let v = e.try_u32()?.to_le_bytes();
        chunk.copy_from_slice(&v[..chunk.len()]);
    }
    Ok(())
}

/// Deterministic and failing sources for host tests.
#[cfg(any(test, feature = "testing"))]
pub mod testing {
    use super::*;

    /// xorshift32: deterministic stand-in for the TRNG.
    pub struct XorShift(pub u32);
    impl Entropy for XorShift {
        fn try_u32(&mut self) -> Result<u32, EntropyError> {
            let mut x = self.0;
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            self.0 = x;
            Ok(x)
        }
    }

    /// Replays a fixed sequence, then fails.
    pub struct Seq(pub Vec<u32>, pub usize);
    impl Entropy for Seq {
        fn try_u32(&mut self) -> Result<u32, EntropyError> {
            let v = self.0.get(self.1).copied().ok_or(EntropyError)?;
            self.1 += 1;
            Ok(v)
        }
    }

    /// Always fails, like a TRNG server that returns an IPC error.
    pub struct Broken;
    impl Entropy for Broken {
        fn try_u32(&mut self) -> Result<u32, EntropyError> { Err(EntropyError) }
    }

    /// Always returns the same word, like a stuck source.
    pub struct Stuck(pub u32);
    impl Entropy for Stuck {
        fn try_u32(&mut self) -> Result<u32, EntropyError> { Ok(self.0) }
    }

    /// Works for `n` draws, then fails.
    pub struct FailAfter(pub XorShift, pub usize);
    impl Entropy for FailAfter {
        fn try_u32(&mut self) -> Result<u32, EntropyError> {
            if self.1 == 0 {
                return Err(EntropyError);
            }
            self.1 -= 1;
            self.0.try_u32()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::testing::*;
    use super::*;

    #[test]
    fn below_stays_in_range() {
        let mut r = XorShift(0x1234_5678);
        for n in [1u32, 2, 3, 10, 26, 88, 2048, u32::MAX] {
            for _ in 0..2000 {
                assert!(below(&mut r, n).unwrap() < n);
            }
        }
    }

    #[test]
    fn below_is_roughly_uniform() {
        let mut r = XorShift(42);
        let mut counts = [0u32; 10];
        for _ in 0..100_000 {
            counts[below(&mut r, 10).unwrap() as usize] += 1;
        }
        for c in counts {
            assert!((9_400..10_600).contains(&c), "{counts:?}");
        }
    }

    #[test]
    fn below_rejects_the_biased_region() {
        // n = 3: threshold = 2^32 mod 3 = 1, so a draw of 0 (low word 0) is rejected.
        let mut r = Seq(vec![0, u32::MAX], 0);
        assert_eq!(below(&mut r, 3), Ok(2));
        assert_eq!(r.1, 2, "first draw should have been rejected");
    }

    #[test]
    fn broken_source_is_an_error_not_zero() {
        // Regression: the old Rng::u32() was get_u32().unwrap_or(0).
        assert_eq!(below(&mut Broken, 10), Err(EntropyError));
        let mut buf = [0xAAu8; 8];
        assert_eq!(fill(&mut Broken, &mut buf), Err(EntropyError));
    }

    #[test]
    fn stuck_source_cannot_spin_forever() {
        // 0 always lands in the rejected region for n = 3.
        assert_eq!(below(&mut Stuck(0), 3), Err(EntropyError));
    }

    #[test]
    fn shuffle_is_a_permutation_and_fails_closed() {
        let mut v: Vec<u32> = (0..50).collect();
        shuffle(&mut XorShift(7), &mut v).unwrap();
        let mut s = v.clone();
        s.sort();
        assert_eq!(s, (0..50).collect::<Vec<_>>());
        assert_ne!(v, s);
        assert_eq!(shuffle(&mut Broken, &mut v), Err(EntropyError));
    }

    #[test]
    fn fill_handles_odd_lengths() {
        let mut r = Seq(vec![0x0403_0201, 0x0807_0605], 0);
        let mut b = [0u8; 6];
        fill(&mut r, &mut b).unwrap();
        assert_eq!(b, [1, 2, 3, 4, 5, 6]);
    }
}
