//! Fixed-point trigonometry and geometry for drawing apps.
//!
//! Replaces the hand-rolled sine tables in the generative apps, which had a
//! shared bug (audit X1: `sin(0) = cos(0) = 0`, so figures collapsed onto
//! their centres and diagonals). Everything here is integer-only, `no_std`
//! friendly and tested against `f64`.
//!
//! * Angles are in [`TURN`] units per full circle (4096; 1 unit ≈ 0.088°).
//!   Angles wrap, and negative angles work.
//! * [`sin`]/[`cos`] return Q16.16 (`ONE` = 65536 = 1.0).
//! * [`polar`] and [`rotate`] work in whole pixels with rounding.
//! * [`clip_line`] clips a segment to a rectangle (Cohen–Sutherland), so
//!   figures larger than the screen are cut at the edge instead of losing
//!   whole segments.

/// Angle units per full turn.
pub const TURN: i32 = 4096;
/// Quarter turn (90°).
pub const QUARTER: i32 = TURN / 4;
/// 1.0 in Q16.16.
pub const ONE: i32 = 1 << 16;

const Q30: i64 = 1 << 30;

/// sin(i * π/2 / 1024) in Q16.16 for i in 0..=1024, computed at compile time
/// with a Taylor series in Q30 (error < 1e-9 before rounding).
const SIN_QUARTER: [i32; (QUARTER as usize) + 1] = build_table();

const fn build_table() -> [i32; (QUARTER as usize) + 1] {
    // π/2 in Q30.
    const HALF_PI_Q30: i64 = 1_686_629_713;
    let mut t = [0i32; (QUARTER as usize) + 1];
    let mut i = 0;
    while i <= QUARTER as usize {
        let x = HALF_PI_Q30 * i as i64 / QUARTER as i64; // Q30
        let x2 = x * x / Q30;
        // sin x = x - x^3/3! + x^5/5! - ... ; term_{k+1} = -term_k * x^2 / ((2k+2)(2k+3))
        let mut term = x;
        let mut sum = x;
        let mut k = 1;
        while k <= 8 {
            term = -term * x2 / Q30 / ((2 * k) * (2 * k + 1)) as i64;
            sum += term;
            k += 1;
        }
        // Q30 -> Q16 with rounding.
        t[i] = ((sum + (1 << 13)) >> 14) as i32;
        i += 1;
    }
    t
}

/// sin(angle) in Q16.16; `angle` in [`TURN`] units (any i32, wraps).
pub fn sin(angle: i32) -> i32 {
    let a = angle.rem_euclid(TURN);
    match a / QUARTER {
        0 => SIN_QUARTER[a as usize],
        1 => SIN_QUARTER[(2 * QUARTER - a) as usize],
        2 => -SIN_QUARTER[(a - 2 * QUARTER) as usize],
        _ => -SIN_QUARTER[(TURN - a) as usize],
    }
}

/// cos(angle) in Q16.16.
pub fn cos(angle: i32) -> i32 { sin(angle + QUARTER) }

/// Multiply an integer by a Q16.16 value, rounding to nearest.
pub fn mul(v: i32, q: i32) -> i32 { (((v as i64) * (q as i64) + (1 << 15)) >> 16) as i32 }

/// Angle of `k` out of `n` equal steps around the circle, without the
/// accumulated truncation of `k * (TURN / n)` (audit: 6-fold symmetry left a
/// 1.4° seam). `n` must be non-zero.
pub fn step_angle(k: i32, n: i32) -> i32 { ((k as i64 * TURN as i64) / n as i64) as i32 }

/// Point at distance `r` (pixels) and `angle` from `center`. Screen
/// coordinates: angle 0 points right, angles increase clockwise (y down).
pub fn polar(center: (i32, i32), r: i32, angle: i32) -> (i32, i32) {
    (center.0 + mul(r, cos(angle)), center.1 + mul(r, sin(angle)))
}

/// Rotate `p` around `center` by `angle`.
pub fn rotate(p: (i32, i32), center: (i32, i32), angle: i32) -> (i32, i32) {
    let (dx, dy) = ((p.0 - center.0) as i64, (p.1 - center.1) as i64);
    let (c, s) = (cos(angle) as i64, sin(angle) as i64);
    let x = (dx * c - dy * s + (1 << 15)) >> 16;
    let y = (dx * s + dy * c + (1 << 15)) >> 16;
    (center.0 + x as i32, center.1 + y as i32)
}

/// Integer square root (floor).
pub fn isqrt(n: u64) -> u64 {
    if n < 2 {
        return n;
    }
    // Newton's method from an upper bound; integer-only (no FPU on RV32IMAC).
    let mut x = 1u64 << ((64 - n.leading_zeros()).div_ceil(2));
    loop {
        let y = (x + n / x) / 2;
        if y >= x {
            return x;
        }
        x = y;
    }
}

/// Clip segment `a`–`b` to the inclusive rectangle `min`..=`max`.
/// Returns `None` if the segment lies entirely outside.
pub fn clip_line(
    mut a: (i32, i32),
    mut b: (i32, i32),
    min: (i32, i32),
    max: (i32, i32),
) -> Option<((i32, i32), (i32, i32))> {
    const L: u8 = 1;
    const R: u8 = 2;
    const T: u8 = 4;
    const B: u8 = 8;
    let code = |p: (i32, i32)| {
        let mut c = 0;
        if p.0 < min.0 {
            c |= L
        } else if p.0 > max.0 {
            c |= R
        }
        if p.1 < min.1 {
            c |= T
        } else if p.1 > max.1 {
            c |= B
        }
        c
    };
    let (mut ca, mut cb) = (code(a), code(b));
    loop {
        if ca | cb == 0 {
            return Some((a, b));
        }
        if ca & cb != 0 {
            return None;
        }
        let out = if ca != 0 { ca } else { cb };
        let (x0, y0, x1, y1) = (a.0 as i64, a.1 as i64, b.0 as i64, b.1 as i64);
        let p = if out & T != 0 {
            let y = min.1 as i64;
            (x0 + (x1 - x0) * (y - y0) / (y1 - y0), y)
        } else if out & B != 0 {
            let y = max.1 as i64;
            (x0 + (x1 - x0) * (y - y0) / (y1 - y0), y)
        } else if out & R != 0 {
            let x = max.0 as i64;
            (x, y0 + (y1 - y0) * (x - x0) / (x1 - x0))
        } else {
            let x = min.0 as i64;
            (x, y0 + (y1 - y0) * (x - x0) / (x1 - x0))
        };
        let p = (p.0 as i32, p.1 as i32);
        if out == ca {
            a = p;
            ca = code(a);
        } else {
            b = p;
            cb = code(b);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rad(a: i32) -> f64 { a as f64 * core::f64::consts::TAU / TURN as f64 }

    #[test]
    fn sin_cos_match_f64_everywhere() {
        for a in -TURN..=2 * TURN {
            let s = (rad(a).sin() * ONE as f64).round() as i32;
            let c = (rad(a).cos() * ONE as f64).round() as i32;
            assert!((sin(a) - s).abs() <= 1, "sin({a}) = {} want {s}", sin(a));
            assert!((cos(a) - c).abs() <= 1, "cos({a}) = {} want {c}", cos(a));
        }
    }

    /// Regression for audit X1: the old tables returned 0 for both.
    #[test]
    fn cardinal_values_exact() {
        assert_eq!((sin(0), cos(0)), (0, ONE));
        assert_eq!((sin(QUARTER), cos(QUARTER)), (ONE, 0));
        assert_eq!((sin(2 * QUARTER), cos(2 * QUARTER)), (0, -ONE));
        assert_eq!((sin(-QUARTER), cos(TURN)), (-ONE, ONE));
    }

    #[test]
    fn polar_hexagon_has_six_distinct_points_at_radius() {
        let c = (168, 260);
        let pts: Vec<_> = (0..6).map(|k| polar(c, 120, step_angle(k, 6))).collect();
        for (i, p) in pts.iter().enumerate() {
            let d = isqrt(((p.0 - c.0).pow(2) + (p.1 - c.1).pow(2)) as u64);
            assert!((119..=120).contains(&d), "{p:?} at {d}");
            assert!(!pts[..i].contains(p));
        }
        assert_eq!(pts[0], (288, 260));
    }

    #[test]
    fn step_angle_has_no_seam() {
        for n in 1..=16 {
            assert_eq!(step_angle(n, n), TURN);
            assert_eq!(step_angle(0, n), 0);
        }
        assert_eq!(step_angle(1, 6), 682);
    }

    #[test]
    fn rotate_quarter_turn() {
        assert_eq!(rotate((10, 0), (0, 0), QUARTER), (0, 10));
        assert_eq!(rotate((5, 5), (5, 5), 1234), (5, 5));
    }

    #[test]
    fn isqrt_exact() {
        for n in 0..10_000u64 {
            let r = isqrt(n);
            assert!(r * r <= n && (r + 1) * (r + 1) > n);
        }
        assert_eq!(isqrt(u32::MAX as u64 * u32::MAX as u64), u32::MAX as u64);
    }

    #[test]
    fn clip_line_cases() {
        let (lo, hi) = ((0, 0), (99, 99));
        assert_eq!(clip_line((10, 10), (20, 20), lo, hi), Some(((10, 10), (20, 20))));
        assert_eq!(clip_line((-50, 50), (150, 50), lo, hi), Some(((0, 50), (99, 50))));
        assert_eq!(clip_line((-10, -10), (-1, 200), lo, hi), None);
        assert_eq!(clip_line((50, -100), (50, 300), lo, hi), Some(((50, 0), (50, 99))));
        let ((x0, y0), (x1, y1)) = clip_line((-100, -100), (200, 200), lo, hi).unwrap();
        assert_eq!((x0, y0, x1, y1), (0, 0, 99, 99));
    }
}
