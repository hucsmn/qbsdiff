use rand::distributions::uniform::{SampleUniform, Uniform};
use rand::prelude::*;

use super::{RhsaSearch, RollingHash, SaSearch, SearchContext, UpgradableSearch};

/// Test the adler-32 rolling hash function.
#[test]
fn rolling_hash() {
    for &n in [0, 1, 7, 32, 97, 1023, 4096].iter() {
        for &chunk in [1, 8, 12, 32, 41].iter().filter(|&&c| c <= n) {
            for &chaos in [0.0, 0.3, 0.7, 1.0].iter() {
                let s = cyclic_bytes(n, chunk, chaos);
                let rh0 = RollingHash::new(&s[..], chunk);
                for (i, h) in rh0.enumerate() {
                    let mut rh1 = RollingHash::new(&s[..], chunk);
                    rh1.forth(i);
                    assert_eq!(Some(h), rh1.hash());

                    let rh2 = RollingHash::new(&s[i..], chunk);
                    assert_eq!(Some(h), rh2.hash());
                }
            }
        }
    }
}

/// Test searching using SaSearch/RhsaSearch.
#[test]
fn rhsa_sa() {
    for &n in [0, 1, 32, 128].iter() {
        for &chunk in [0, 1, 8, 32, 41].iter().filter(|&&c| c <= n || c == 0) {
            let chunk = Ord::max(1, chunk);
            let s = cyclic_bytes(n, chunk, 0.2);

            for &similar in [0.0, 0.5, 1.0].iter() {
                let t = distort(&s[..], similar);
                let mut sa = SaSearch::new(&s[..], &t[..], chunk);
                let mut rhsa = RhsaSearch::new(&s[..], &t[..], chunk);

                assert_eq!(sa.len(), rhsa.len());
                for i in 0..sa.len() {
                    assert_eq!(sa.cursor(), i);
                    assert_eq!(rhsa.cursor(), i);

                    let (mut j, mut n) = sa.search();
                    if n < chunk {
                        j = s.len();
                        n = 0;
                    }
                    let (k, m) = rhsa.search();

                    let s0 = &s[j..j + n];
                    let s1 = &s[k..k + m];
                    let t0 = &t[i..i + n];
                    let t1 = &t[i..i + m];
                    assert_eq!(n, m);
                    assert_eq!(s0, t0);
                    assert_eq!(s1, t1);
                    assert_eq!(s0, s1);

                    sa.forth(1);
                    rhsa.forth(1);
                }
            }
        }
    }
}

/// Test UpgradableSearch.
#[test]
fn sa_upgrade() {
    for &n in [0, 1, 32, 128].iter() {
        for &chunk in [0, 1, 8, 32, 41].iter().filter(|&&c| c <= n || c == 0) {
            let chunk = Ord::max(1, chunk);
            let s = cyclic_bytes(n, chunk, 0.2);

            for &similar in [0.0, 0.5, 1.0].iter() {
                let t = distort(&s[..], similar);
                let mut us = UpgradableSearch::new(&s[..], &t[..], chunk);
                let mut rhsa = RhsaSearch::new(&s[..], &t[..], chunk);

                assert_eq!(us.len(), rhsa.len());
                let upgrade_point = us.len() / 2;
                for i in 0..us.len() {
                    assert_eq!(us.cursor(), i);
                    assert_eq!(rhsa.cursor(), i);

                    if i >= upgrade_point && !us.is_upgraded() {
                        us = us.upgrade();
                    }

                    let (mut j, mut n) = us.search();
                    if n < chunk {
                        j = s.len();
                        n = 0;
                    }
                    let (k, m) = rhsa.search();

                    let s0 = &s[j..j + n];
                    let s1 = &s[k..k + m];
                    let t0 = &t[i..i + n];
                    let t1 = &t[i..i + m];
                    assert_eq!(n, m);
                    assert_eq!(s0, t0);
                    assert_eq!(s1, t1);
                    assert_eq!(s0, s1);

                    us.forth(1);
                    rhsa.forth(1);
                }
            }
        }
    }
}

/// Generate source samples.
fn cyclic_bytes(n: usize, cycle: usize, chaos: f64) -> Vec<u8> {
    let cycle = Ord::max(cycle, 1);
    let chaos = fraction(chaos);
    let collision = if 0.1 * chaos > 0.5 { 0.5 } else { 0.1 * chaos };
    let pat_samples = Ord::max(1, ((n / cycle) as f64 * chaos) as usize);
    let pat_count = random_between(0, (pat_samples as f64 * (1.0 - chaos)) as usize);
    let junk_size = n - cycle * pat_count;
    let junk_piece = Ord::max(1, Ord::min(cycle * 2, (junk_size as f64 * 0.1) as usize));

    let mut pats = Vec::with_capacity(pat_samples);
    while pats.len() < pat_samples {
        let pat = random_bytes(cycle);
        pats.push(pat.clone());
        while random_decide(collision) && pats.len() < pat_samples {
            let cpat = adler32_collision(&pat[..]);
            pats.push(cpat);
        }
    }

    let mut j = junk_size;
    let mut p = pat_count;
    let mut bytes = Vec::with_capacity(n);
    while bytes.len() < n {
        if j > 0 && random_decide(j as f64 / n as f64) {
            let piece = random_bytes(random_between(0, Ord::min(j, junk_piece)));
            bytes.extend_from_slice(&piece[..]);
            j -= piece.len();
        } else if p > 0 && n - bytes.len() >= cycle {
            let i = random_between(0, pats.len() - 1);
            bytes.extend_from_slice(&pats[i][..]);
            p -= 1;
        } else {
            let piece = random_bytes(n - bytes.len());
            bytes.extend_from_slice(&piece[..]);
        }
    }

    bytes
}

/// Try to find adler-32 collision.
fn adler32_collision(pat: &[u8]) -> Vec<u8> {
    let n = pat.len();
    if n < 3 || n > 128 {
        return Vec::from(pat);
    }

    let i = 0;
    let j = n / 2;
    let k = n - 1;

    let t = gcd(n - 1, n - j - 1);
    let a = (t * (n - j - 1)) as isize;
    let b = -((t * (n - 1)) as isize);
    let c = -a - b;

    let mut v = Vec::from(pat);
    if try_add(&mut v[i], a) && try_add(&mut v[j], b) && try_add(&mut v[k], c) {
        return v;
    }

    let mut v = Vec::from(pat);
    let a = -a;
    let b = -b;
    let c = -c;
    if try_add(&mut v[i], a) && try_add(&mut v[j], b) && try_add(&mut v[k], c) {
        return v;
    }

    Vec::from(pat)
}

fn try_add(x: &mut u8, d: isize) -> bool {
    if d >= 0 && d <= (std::u8::MAX - *x) as isize {
        *x += d as u8;
        true
    } else if d < 0 && -d <= *x as isize {
        *x -= (-d) as u8;
        true
    } else {
        false
    }
}

fn gcd(mut x: usize, mut y: usize) -> usize {
    while x != 0 {
        let z = x;
        x = y % x;
        y = z;
    }
    y
}

fn random_bytes(n: usize) -> Vec<u8> {
    let mut rng = thread_rng();
    let mut bytes = Vec::with_capacity(n);
    for _ in 0..n {
        bytes.push(rng.gen())
    }
    bytes
}

/// Generate target from source samples.
fn distort(source: &[u8], similar: f64) -> Vec<u8> {
    let similar = fraction(similar);
    let rate = convex_mapping(similar);

    let tsize = random_between(
        (source.len() as f64 * 0.75) as usize,
        (source.len() as f64 * 1.25) as usize,
    );
    let dmax = random_between(
        Ord::min(16, (source.len() as f64 * 0.02) as usize),
        Ord::max(32, (source.len() as f64 * 0.33) as usize),
    );
    let emax = random_between(0, (source.len() as f64 * 0.15 * (1.0 - similar)) as usize);

    let mut target = Vec::with_capacity(tsize);
    let mut rng = thread_rng();
    while target.len() < tsize {
        // delta
        let remain = tsize - target.len();
        let dsize = {
            let dhi = Ord::min(Ord::min(dmax, remain), source.len());
            let dlo = Ord::min(16, dhi);
            random_between(dlo, dhi)
        };
        let offset = random_between(0, source.len() - dsize);
        for &x in source[offset..offset + dsize].iter() {
            if random_decide(rate) {
                target.push(x);
            } else {
                target.push(rng.gen());
            }
        }

        // extra
        let remain = tsize - target.len();
        if !random_decide(rate) {
            let esize = random_between(0, Ord::min(emax, remain));
            for _ in 0..esize {
                target.push(rng.gen());
            }
        }
    }

    target
}

fn random_decide(rate: f64) -> bool {
    random_between(0.0, 1.0) <= fraction(rate)
}

fn random_between<X: SampleUniform>(lo: X, hi: X) -> X {
    let mut rng = thread_rng();
    Uniform::new_inclusive(lo, hi).sample(&mut rng)
}

fn fraction(x: f64) -> f64 {
    if x.is_nan() || x.is_sign_negative() {
        0.0
    } else if x.is_infinite() || x > 1.0 {
        1.0
    } else {
        x
    }
}

fn convex_mapping(frac: f64) -> f64 {
    (1.0 - (1.0 - frac) * (1.0 - frac)).sqrt()
}
