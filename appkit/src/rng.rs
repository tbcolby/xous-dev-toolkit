//! Randomness that fails closed.
//!
//! The pure algorithms ([`unbiased_below`], [`shuffle`]) take any
//! [`RandomSource`] so they are testable on the host. On Xous, [`Trng`] wraps
//! the hardware TRNG and **panics** if the TRNG server returns an error.
//! A crash is recoverable; a key, pad, or deck generated from a constant is not.

/// Something that yields uniformly random `u32`s.
pub trait RandomSource {
    fn next_u32(&mut self) -> u32;
}

/// Uniform value in `0..n` with no modulo bias (Lemire's method with
/// rejection). Panics if `n == 0`.
pub fn unbiased_below<R: RandomSource + ?Sized>(rng: &mut R, n: u32) -> u32 {
    assert!(n > 0, "unbiased_below: n must be > 0");
    let mut m = (rng.next_u32() as u64) * (n as u64);
    let mut low = m as u32;
    if low < n {
        let threshold = n.wrapping_neg() % n;
        while low < threshold {
            m = (rng.next_u32() as u64) * (n as u64);
            low = m as u32;
        }
    }
    (m >> 32) as u32
}

/// Uniform value in the inclusive range `lo..=hi`.
pub fn range_inclusive<R: RandomSource + ?Sized>(rng: &mut R, lo: u32, hi: u32) -> u32 {
    assert!(lo <= hi, "range_inclusive: lo > hi");
    let span = hi - lo;
    if span == u32::MAX {
        return rng.next_u32();
    }
    lo + unbiased_below(rng, span + 1)
}

/// Fisher-Yates shuffle.
pub fn shuffle<T, R: RandomSource + ?Sized>(rng: &mut R, items: &mut [T]) {
    for i in (1..items.len()).rev() {
        let j = unbiased_below(rng, (i + 1) as u32) as usize;
        items.swap(i, j);
    }
}

/// Fill a byte slice.
pub fn fill_bytes<R: RandomSource + ?Sized>(rng: &mut R, dest: &mut [u8]) {
    for chunk in dest.chunks_mut(4) {
        let v = rng.next_u32().to_le_bytes();
        chunk.copy_from_slice(&v[..chunk.len()]);
    }
}

/// Hardware TRNG, fail-closed.
#[cfg(feature = "xous")]
pub struct Trng {
    inner: trng::Trng,
}

#[cfg(feature = "xous")]
impl Trng {
    pub fn new(xns: &xous_names::XousNames) -> Result<Self, xous::Error> {
        Ok(Self { inner: trng::Trng::new(xns)? })
    }

    /// Fallible access, for callers that want to show an error instead of panicking.
    pub fn try_u32(&self) -> Result<u32, xous::Error> { self.inner.get_u32() }

    pub fn below(&mut self, n: u32) -> u32 { unbiased_below(self, n) }

    pub fn range_inclusive(&mut self, lo: u32, hi: u32) -> u32 { range_inclusive(self, lo, hi) }

    pub fn shuffle<T>(&mut self, items: &mut [T]) { shuffle(self, items) }

    pub fn fill_bytes(&mut self, dest: &mut [u8]) { fill_bytes(self, dest) }
}

#[cfg(feature = "xous")]
impl RandomSource for Trng {
    fn next_u32(&mut self) -> u32 {
        match self.inner.get_u32() {
            Ok(v) => v,
            Err(e) => panic!("TRNG failure ({:?}); refusing to continue with non-random data", e),
        }
    }
}

/// Lets core crates written against the fallible
/// [`precursor_appkit_core::entropy::Entropy`] trait use the hardware TRNG.
/// Errors become `EntropyError` (no panic), so the caller can show a message.
#[cfg(feature = "xous")]
impl precursor_appkit_core::entropy::Entropy for Trng {
    fn try_u32(&mut self) -> Result<u32, precursor_appkit_core::entropy::EntropyError> {
        self.inner.get_u32().map_err(|_| precursor_appkit_core::entropy::EntropyError)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// xorshift32: deterministic test source.
    struct XorShift(u32);
    impl RandomSource for XorShift {
        fn next_u32(&mut self) -> u32 {
            let mut x = self.0;
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            self.0 = x;
            x
        }
    }

    /// Replays a fixed sequence.
    struct Seq(Vec<u32>, usize);
    impl RandomSource for Seq {
        fn next_u32(&mut self) -> u32 {
            let v = self.0[self.1 % self.0.len()];
            self.1 += 1;
            v
        }
    }

    #[test]
    fn stays_in_range() {
        let mut r = XorShift(0x1234_5678);
        for n in [1u32, 2, 3, 6, 7, 52, 1000, u32::MAX] {
            for _ in 0..2000 {
                assert!(unbiased_below(&mut r, n) < n);
            }
        }
    }

    #[test]
    fn roughly_uniform_d6() {
        let mut r = XorShift(42);
        let mut counts = [0u32; 6];
        for _ in 0..60_000 {
            counts[unbiased_below(&mut r, 6) as usize] += 1;
        }
        for c in counts {
            assert!((9_000..11_000).contains(&c), "{:?}", counts);
        }
    }

    #[test]
    fn rejects_biased_region() {
        // n = 3: threshold = (2^32 - 3) % 3 = 1. A draw whose low word is 0 must be rejected.
        // next_u32() = 0 -> m = 0, low = 0 < threshold -> reject; then u32::MAX -> 2.
        let mut r = Seq(vec![0, u32::MAX], 0);
        assert_eq!(unbiased_below(&mut r, 3), 2);
        assert_eq!(r.1, 2, "first draw should have been rejected");
    }

    #[test]
    fn inclusive_range_full_span() {
        let mut r = Seq(vec![7], 0);
        assert_eq!(range_inclusive(&mut r, 0, u32::MAX), 7);
        let mut r = XorShift(9);
        for _ in 0..1000 {
            let v = range_inclusive(&mut r, 1, 6);
            assert!((1..=6).contains(&v));
        }
    }

    #[test]
    fn shuffle_is_permutation() {
        let mut r = XorShift(77);
        let mut v: Vec<u32> = (0..52).collect();
        shuffle(&mut r, &mut v);
        let mut s = v.clone();
        s.sort();
        assert_eq!(s, (0..52).collect::<Vec<_>>());
        assert_ne!(v, (0..52).collect::<Vec<_>>());
    }

    #[test]
    fn fill_bytes_odd_length() {
        let mut r = Seq(vec![0x0403_0201], 0);
        let mut b = [0u8; 6];
        fill_bytes(&mut r, &mut b);
        assert_eq!(b, [1, 2, 3, 4, 1, 2]);
    }

    #[test]
    #[should_panic]
    fn zero_bound_panics() {
        let mut r = XorShift(1);
        unbiased_below(&mut r, 0);
    }
}
