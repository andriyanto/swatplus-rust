//! Small SWAT+ routines everything else depends on: the random number generator and its
//! distributions (`aunif`, `cli_dstn1`, `atri`), and the name lookup (`search`).
//!
//! Arithmetic is single precision in the order the Fortran writes it, so results match bit for
//! bit where the math library agrees.

use std::cmp::Ordering;

/// Fortran `x ** y` for a real exponent: the math library's `powf`, as gfortran calls it.
/// Rust's `x.powf(0.5)` with a constant exponent is turned into `sqrt` by the optimizer, which is
/// correctly rounded where `powf` is not always (they differ in the last bit for about 1 value
/// in 1600); hiding the exponent from the optimizer keeps the library call.
pub fn rpow(x: f32, y: f32) -> f32 {
    x.powf(std::hint::black_box(y))
}

/// The seed `aunif` leaves after `n` draws from `seed`, without drawing them: `aunif` is the
/// Park–Miller generator, `seed · 16807ⁿ mod (2³¹ − 1)`.
pub fn aunif_skip(seed: i32, n: u64) -> i32 {
    const M: u64 = 2_147_483_647;
    let (mut base, mut e, mut r) = (16_807_u64, n, 1_u64);
    while e > 0 {
        if e & 1 == 1 {
            r = r * base % M;
        }
        base = base * base % M;
        e >>= 1;
    }
    ((u64::from(seed.unsigned_abs()) % M) * r % M) as i32
}

/// SWAT+ `aunif`: the Park–Miller minimal standard generator (Schrage's method). Advances `seed`
/// and returns a number in (0, 1). The product is formed in double precision and rounded to single,
/// as `x1 * 4.656612875d-10` assigned to a `real`.
pub fn aunif(seed: &mut i32) -> f32 {
    let x2 = *seed / 127_773;
    *seed = 16_807 * (*seed - x2 * 127_773) - x2 * 2836;
    if *seed < 0 {
        *seed += 2_147_483_647;
    }
    (f64::from(*seed) * 4.656612875e-10) as f32
}

/// SWAT+ `cli_dstn1`: a standard normal deviate from two uniform numbers (Box–Muller).
pub fn dstn1(rn1: f32, rn2: f32) -> f32 {
    (-2.0_f32 * rn1.ln()).sqrt() * (6.283185_f32 * rn2).cos()
}

/// SWAT+ `atri`: a value from a triangular distribution with limits `at1`, `at3` and mean `at2`,
/// scaled to that mean and kept within (0, 1). Advances `seed`.
pub fn atri(at1: f32, at2: f32, at3: f32, seed: &mut i32) -> f32 {
    let u3 = at2 - at1;
    let rn = aunif(seed);
    let y = 2.0_f32 / (at3 - at1);
    let b2 = at3 - at2;
    let b1 = rn / y;
    let x1 = y * u3 / 2.0;
    let mut atri = if rn <= x1 {
        let xx = 2.0_f32 * b1 * u3;
        let yy = if xx <= 0.0 { 0.0 } else { xx.sqrt() };
        yy + at1
    } else {
        let xx = b2 * b2 - 2.0_f32 * b2 * (b1 - 0.5 * u3);
        let yy = if xx <= 0.0 { 0.0 } else { xx.sqrt() };
        at3 - yy
    };
    let amn = (at3 + at2 + at1) / 3.0;
    atri = atri * at2 / amn;
    if atri >= 1.0 {
        atri = 0.99;
    }
    if atri <= 0.0 {
        atri = 0.001;
    }
    atri
}

/// Compares two names as Fortran compares `character(len=50)` values: cut to 50 characters and
/// padded with blanks.
pub fn fortran_cmp(a: &str, b: &str) -> Ordering {
    const LEN: usize = 50;
    let a = a.as_bytes();
    let b = b.as_bytes();
    for i in 0..LEN {
        let x = a.get(i).copied().unwrap_or(b' ');
        let y = b.get(i).copied().unwrap_or(b' ');
        match x.cmp(&y) {
            Ordering::Equal => {}
            other => return other,
        }
    }
    Ordering::Equal
}

/// SWAT+ `search`: a binary search of `list` for `find`, giving its 1-based position. It assumes
/// a sorted list and gives up after 25 halvings, so in an unsorted list a name that is there can
/// be missed. `None` is "not found": SWAT+ then leaves the caller's value as it was.
pub fn search(list: &[String], find: &str) -> Option<usize> {
    if list.is_empty() {
        return None;
    }
    let eq = |i: usize| list.get(i - 1).is_some_and(|s| fortran_cmp(s, find) == Ordering::Equal);
    let cmp = |i: usize| list.get(i - 1).map(|s| fortran_cmp(s, find));
    let mut nf = 1usize;
    let mut nl = list.len();
    for _ in 0..25 {
        let nn = (nl - nf) / 2 + nf;
        if nl - nf == 1 {
            if eq(nl) {
                return Some(nl);
            }
            if eq(nf) {
                return Some(nf);
            }
        }
        if eq(nn) {
            return Some(nn);
        }
        if cmp(nn) == Some(Ordering::Greater) {
            nl = nn;
        }
        if cmp(nn) == Some(Ordering::Less) {
            nf = nn;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    #[test]
    fn aunif_skip_is_aunif_drawn() {
        for start in [1, 748_932_582, 64_298_628, 2_147_483_646] {
            let mut seed = start;
            for n in 0..2000_u64 {
                assert_eq!(super::aunif_skip(start, n), seed, "{start} after {n}");
                super::aunif(&mut seed);
            }
        }
    }

    use super::*;

    #[test]
    fn park_miller() {
        // The minimal standard generator: seed 1 gives 16807, then 282475249.
        let mut s = 1;
        aunif(&mut s);
        assert_eq!(s, 16_807);
        aunif(&mut s);
        assert_eq!(s, 282_475_249);
        // The 10 000th value from seed 1 is 1043618065 (Park and Miller, 1988).
        let mut s = 1;
        for _ in 0..10_000 {
            aunif(&mut s);
        }
        assert_eq!(s, 1_043_618_065);
    }

    #[test]
    fn binary_search_quirks() {
        let sorted: Vec<String> = ["a.pcp", "b.pcp", "c.pcp", "d.pcp"].iter().map(|s| s.to_string()).collect();
        for (i, name) in sorted.iter().enumerate() {
            assert_eq!(search(&sorted, name), Some(i + 1));
        }
        assert_eq!(search(&sorted, "x.pcp"), None);
        // Ames_sub1: the station names ames.tem, the list holds ames.tmp.
        assert_eq!(search(&["ames.tmp".to_string()], "ames.tem"), None);
        // Unsorted: found when the first probe lands on it...
        let unsorted: Vec<String> = ["z", "y", "a", "x", "w"].iter().map(|s| s.to_string()).collect();
        assert_eq!(search(&unsorted, "a"), Some(3));
        // ...but missed when the halving walks away from it.
        let unsorted: Vec<String> = ["b", "c", "a"].iter().map(|s| s.to_string()).collect();
        assert_eq!(search(&unsorted, "a"), None);
    }

    #[test]
    fn padded_comparison() {
        assert_eq!(fortran_cmp("abc", "abc   "), Ordering::Equal);
        // A blank (32) sorts before '!' (33).
        assert_eq!(fortran_cmp("ab", "ab!"), Ordering::Less);
    }

    #[test]
    fn atri_stays_in_bounds() {
        let mut seed = 1_767_585_417;
        for _ in 0..1000 {
            let v = atri(0.2, 0.6, 0.9, &mut seed);
            assert!(v > 0.0 && v < 1.0);
        }
    }
}
