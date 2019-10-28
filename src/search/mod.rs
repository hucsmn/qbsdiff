#![allow(dead_code)]

/// The motivation of rolling hash accelerated suffix array (RHSA) is that
/// bad performance on some special samples have been noticed:
/// 
/// Execution time of qbsdiff and bsdiff on qemu-m68k binaries (about 2M) is
/// too horribly long (about 17min on i5-4200H @ 2.80GHz),
/// while other binaries from 1M to 100M usually finished in just 1s to 30s.
/// 
/// With the help of flamegraph, the bottle neck is determined:
/// bsdiff algorithm executes SuffixArray::search_lcp for too many times in
/// those samples.
/// 
/// This branch attempts to accelerate suffix array searching with a table that
/// cached rolling hash.
/// However, the result of performance is disappointing: 
/// 
/// Execution time on small binaries (< 10M) is usually two times longer, while
/// performance on special samples (qemu-m68k binaries) shows no significant
/// improvements.
/// 
/// Therefore, bad performance of qbsdiff on qemu-m68k binaries is suspected to
/// be resulted from the greed nature of bsdiff algorithm or the characteristics
/// of those binaries.
/// Maybe, new algorithm should be designed to better handle those bad cases.
/// 
/// This branch of the experimental RHSA implementation is therefore deprecated.

use adler32::RollingAdler32;
use std::collections::HashMap;
use std::ops::Range;
use suffix_array::SuffixArray;

#[cfg(test)]
mod tests;

/// Place holder for RhsaSearch building tricks.
const UNDEFINED_OFFSET: u32 = 0xffffffff;

/// Bsdiff search context.
pub trait SearchContext {
    /// The source.
    fn source(&self) -> &[u8];

    /// The target.
    fn target(&self) -> &[u8];

    /// Minimum length of match.
    fn min_match(&self) -> usize;

    /// Maximum target cursor.
    fn len(&self) -> usize;

    /// Get the target cursor.
    fn cursor(&self) -> usize;

    /// Forth the target cursor.
    fn forth(&mut self, n: usize) -> usize;

    /// Search for a LCP of the remaining target in the source.
    ///
    /// The matches smaller than `min_match()` could possibly be ignored.
    fn search(&self) -> (usize, usize);
}

/// Upgradable search context.
pub enum UpgradableSearch<'s, 't> {
    Sa(SaSearch<'s, 't>),
    Rhsa(RhsaSearch<'s, 't>),
}

impl<'s, 't> UpgradableSearch<'s, 't> {
    /// Create upgradable search context.
    pub fn new(s: &'s [u8], t: &'t [u8], small: usize) -> Self {
        UpgradableSearch::Sa(SaSearch::new(s, t, small))
    }

    /// Test if upgraded.
    pub fn is_upgraded(&self) -> bool {
        match self {
            UpgradableSearch::Sa(_) => false,
            _ => true,
        }
    }

    /// Force upgrade.
    pub fn upgrade(self) -> Self {
        match self {
            UpgradableSearch::Sa(sa) => UpgradableSearch::Rhsa(sa.into()),
            _ => self,
        }
    }
}

impl<'s, 't> SearchContext for UpgradableSearch<'s, 't> {
    fn source(&self) -> &[u8] {
        match self {
            UpgradableSearch::Sa(sa) => sa.source(),
            UpgradableSearch::Rhsa(rhsa) => rhsa.source(),
        }
    }

    fn target(&self) -> &[u8] {
        match self {
            UpgradableSearch::Sa(sa) => sa.target(),
            UpgradableSearch::Rhsa(rhsa) => rhsa.target(),
        }
    }

    fn min_match(&self) -> usize {
        match self {
            UpgradableSearch::Sa(sa) => sa.min_match(),
            UpgradableSearch::Rhsa(rhsa) => rhsa.min_match(),
        }
    }

    fn len(&self) -> usize {
        match self {
            UpgradableSearch::Sa(sa) => sa.len(),
            UpgradableSearch::Rhsa(rhsa) => rhsa.len(),
        }
    }

    fn cursor(&self) -> usize {
        match self {
            UpgradableSearch::Sa(sa) => sa.cursor(),
            UpgradableSearch::Rhsa(rhsa) => rhsa.cursor(),
        }
    }

    fn forth(&mut self, n: usize) -> usize {
        match self {
            UpgradableSearch::Sa(sa) => sa.forth(n),
            UpgradableSearch::Rhsa(rhsa) => rhsa.forth(n),
        }
    }

    fn search(&self) -> (usize, usize) {
        match self {
            UpgradableSearch::Sa(sa) => sa.search(),
            UpgradableSearch::Rhsa(rhsa) => rhsa.search(),
        }
    }
}

/// Simple suffix array based search context.
pub struct SaSearch<'s, 't> {
    s: &'s [u8],
    t: &'t [u8],
    sa: SuffixArray<'s>,
    small: usize,
    cursor: usize,
}

impl<'s, 't> SaSearch<'s, 't> {
    /// Create a thin wrapper for suffix_array::SuffixArray.
    pub fn new(s: &'s [u8], t: &'t [u8], small: usize) -> Self {
        SaSearch {
            s,
            t,
            sa: SuffixArray::new(s),
            small,
            cursor: 0,
        }
    }
}

impl<'s, 't> SearchContext for SaSearch<'s, 't> {
    fn source(&self) -> &[u8] {
        self.s
    }

    fn target(&self) -> &[u8] {
        self.t
    }

    fn min_match(&self) -> usize {
        self.small
    }

    fn len(&self) -> usize {
        if let Some(n) = self.t.len().checked_sub(self.small) {
            n + 1
        } else {
            0
        }
    }

    fn cursor(&self) -> usize {
        self.cursor
    }

    fn forth(&mut self, n: usize) -> usize {
        self.cursor += n;
        self.cursor
    }

    fn search(&self) -> (usize, usize) {
        let pat = &self.t[Ord::min(self.cursor, self.t.len())..];
        let Range { start, end } = self.sa.search_lcp(pat);
        (start, end.saturating_sub(start))
    }
}

/// Rolling hash accelerated suffix array search context.
pub struct RhsaSearch<'s, 't> {
    s: &'s [u8],
    chunk: usize,
    tab: HashMap<u32, (u32, u32)>,
    dat: Vec<u32>,
    th: RollingHash<'t>,
}

impl<'s, 't> RhsaSearch<'s, 't> {
    /// Create Rolling hash accelerated suffix array search context.
    pub fn new(s: &'s [u8], t: &'t [u8], chunk: usize) -> Self {
        let (_, sa) = SuffixArray::new(s).into_parts();
        Self::build(s, t, sa, chunk, 0)
    }

    /// Build RhsaSearch。
    fn build(s: &'s [u8], t: &'t [u8], sa: Vec<u32>, chunk: usize, cursor: usize) -> Self {
        if chunk == 0 {
            panic!("empty chunk");
        }

        let mut th = RollingHash::new(t, chunk);
        th.forth(cursor);

        if chunk > s.len() {
            return RhsaSearch {
                s,
                chunk,
                tab: HashMap::new(),
                dat: Vec::new(),
                th,
            };
        }

        let sh = RollingHash::new(s, chunk);
        let mut hs = Vec::with_capacity(sh.len());
        let mut tab = HashMap::new();
        for h in sh {
            hs.push(h);
            if let Some((_, n)) = tab.get_mut(&h) {
                *n += 1;
            } else {
                tab.insert(h, (UNDEFINED_OFFSET, 1));
            }
        }
        tab.shrink_to_fit();

        let mut p = 0;
        let mut dat = Vec::with_capacity(hs.len());
        unsafe {
            dat.set_len(dat.capacity());
        }
        for x in sa.into_iter().filter(|&i| (i as usize) < hs.len()) {
            let h = hs[x as usize];
            if let Some((i, n)) = tab.get_mut(&h) {
                if *i != UNDEFINED_OFFSET {
                    dat[*i as usize] = x;
                    *i += 1;
                } else {
                    dat[p as usize] = x;
                    *i = p + 1;
                    p += *n;
                }
            } else {
                unreachable!();
            }
        }
        for (_, (i, n)) in tab.iter_mut() {
            *i -= *n;
        }

        RhsaSearch {
            s,
            chunk,
            tab,
            dat,
            th,
        }
    }
}

/// Convert SaSearch to RhsaSearch.
impl<'s, 't> From<SaSearch<'s, 't>> for RhsaSearch<'s, 't> {
    fn from(sa_search: SaSearch<'s, 't>) -> Self {
        let s = sa_search.s;
        let t = sa_search.t;
        let (_, sa) = sa_search.sa.into_parts();
        let chunk = Ord::max(sa_search.small, 1);
        let cursor = sa_search.cursor;
        RhsaSearch::build(s, t, sa, chunk, cursor)
    }
}

impl<'s, 't> SearchContext for RhsaSearch<'s, 't> {
    fn source(&self) -> &[u8] {
        self.s
    }

    fn target(&self) -> &[u8] {
        self.th.tape()
    }

    fn min_match(&self) -> usize {
        self.chunk
    }

    fn len(&self) -> usize {
        self.th.len()
    }

    fn cursor(&self) -> usize {
        self.th.cursor()
    }

    fn forth(&mut self, n: usize) -> usize {
        self.th.forth(n)
    }

    fn search(&self) -> (usize, usize) {
        let pat = self.th.remain();

        let h;
        if let Some(hash) = self.th.hash() {
            h = hash;
        } else {
            return (self.s.len(), 0);
        }

        let sa;
        if let Some(&(i, n)) = self.tab.get(&h) {
            sa = &self.dat[i as usize..i as usize + n as usize];
        } else {
            return (self.s.len(), 0);
        }

        let point = sa.binary_search_by(|&i| self.s[i as usize..].cmp(pat));
        let mut offset;
        let mut length;
        match point {
            Ok(i) => {
                offset = sa[i] as usize;
                length = self.s.len() - offset;
            }
            Err(i) => {
                if i == 0 {
                    offset = sa[0] as usize;
                    length = lcp(pat, &self.s[offset..]);
                } else if i < sa.len() {
                    let j = sa[i - 1] as usize;
                    let k = sa[i] as usize;
                    let a = lcp(pat, &self.s[j..]);
                    let b = lcp(pat, &self.s[k..]);
                    if a > b {
                        offset = j;
                        length = a;
                    } else {
                        offset = k;
                        length = b;
                    }
                } else if i == sa.len() {
                    offset = sa[i - 1] as usize;
                    length = lcp(pat, &self.s[offset..]);
                } else {
                    return (self.s.len(), 0);
                }
            }
        }
        if length < self.chunk {
            offset = self.s.len();
            length = 0;
        }
        (offset, length)
    }
}

/// Count the longest common prefix of two strings.
#[inline]
fn lcp(xs: &[u8], ys: &[u8]) -> usize {
    Iterator::zip(xs.iter(), ys.iter())
        .take_while(|(&x, &y)| x == y)
        .count()
}

/// State of the adler-32 rolling hasher.
struct RollingHash<'a> {
    tape: &'a [u8],
    chunk: usize,
    valid: bool,
    cursor: usize,
    adler: RollingAdler32,
}

impl<'a> RollingHash<'a> {
    pub fn new(tape: &'a [u8], chunk: usize) -> Self {
        if chunk == 0 {
            panic!("empty chunk size");
        } else if tape.len() < chunk {
            RollingHash {
                tape,
                chunk,
                valid: false,
                cursor: 0,
                adler: RollingAdler32::new(),
            }
        } else {
            let adler = RollingAdler32::from_buffer(&tape[..chunk]);
            RollingHash {
                tape,
                chunk,
                valid: true,
                cursor: 0,
                adler,
            }
        }
    }

    fn len(&self) -> usize {
        if let Some(n) = self.tape.len().checked_sub(self.chunk) {
            n + 1
        } else {
            0
        }
    }

    fn tape(&self) -> &'a [u8] {
        self.tape
    }

    fn remain(&self) -> &'a [u8] {
        &self.tape[Ord::min(self.cursor, self.tape.len())..]
    }

    fn cursor(&self) -> usize {
        self.cursor
    }

    fn forth(&mut self, mut n: usize) -> usize {
        if self.cursor.saturating_add(n) >= self.len() {
            self.valid = false;
            self.cursor = self.cursor.saturating_add(n);
            return self.cursor;
        }
        if n > self.chunk / 2 {
            self.cursor += n;
            let piece = &self.tape[self.cursor..self.cursor + self.chunk];
            self.adler = RollingAdler32::from_buffer(piece);
            return self.cursor;
        }
        while n > 0 {
            self.adler.remove(self.chunk, self.tape[self.cursor]);
            self.adler.update(self.tape[self.cursor + self.chunk]);
            self.cursor += 1;
            n -= 1;
        }
        self.cursor
    }

    fn hash(&self) -> Option<u32> {
        if self.valid {
            Some(self.adler.hash())
        } else {
            None
        }
    }
}

/// Make RollingHash an iterator of u32.
impl<'a> Iterator for RollingHash<'a> {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(h) = self.hash() {
            self.forth(1);
            Some(h)
        } else {
            None
        }
    }
}
