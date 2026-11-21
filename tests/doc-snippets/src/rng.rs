//! randomness.md snippets
pub struct Rng {
    trng: trng::Trng,
}

impl Rng {
    pub fn new(xns: &xous_names::XousNames) -> Self {
        Self { trng: trng::Trng::new(xns).expect("can't connect to TRNG") }
    }

    /// Fails closed: a TRNG error must never silently become a constant.
    pub fn u32(&self) -> u32 { self.trng.get_u32().expect("TRNG read failed") }

    /// Uniform in 0..max (rejection sampling, no modulo bias).
    pub fn range(&self, max: u32) -> u32 {
        if max <= 1 {
            return 0;
        }
        let threshold = u32::MAX - (u32::MAX % max); // a multiple of max
        loop {
            let val = self.u32();
            if val < threshold {
                return val % max;
            }
        }
    }

    pub fn flip(&self) -> bool { self.u32() & 1 == 1 }

    pub fn shuffle<T>(&self, items: &mut [T]) {
        let n = items.len();
        for i in (1..n).rev() {
            let j = self.range((i + 1) as u32) as usize;
            items.swap(i, j);
        }
    }

    pub fn weighted_pick(&self, weights: &[u32]) -> usize {
        let total: u32 = weights
            .iter()
            .try_fold(0u32, |acc, &w| acc.checked_add(w))
            .expect("weights sum overflows u32");
        if total == 0 {
            return 0;
        }
        let mut roll = self.range(total);
        for (i, &w) in weights.iter().enumerate() {
            if roll < w {
                return i;
            }
            roll -= w;
        }
        weights.len() - 1
    }
}
