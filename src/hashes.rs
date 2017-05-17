// https://www.eecs.harvard.edu/~michaelm/postscripts/tr-02-05.pdf
pub struct Hashes {
    base: u64,
    increment: u64,
}

impl Hashes {
    pub fn new(hash1: u64, hash2: u64) -> Hashes {
        Hashes {
            base: hash1,
            increment: hash2,
        }
    }
}

impl Iterator for Hashes {
    type Item = u64;

    fn next(&mut self) -> Option<u64> {
        let next_value = self.base;
        self.base = self.base.wrapping_add(self.increment);
        Some(next_value)
    }
}
