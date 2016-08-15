use murmur3::murmur3_x64_128;
use std::marker::PhantomData;
use std::io::Cursor;
use byteorder::{BigEndian, ByteOrder};

use hashes::Hashes;
use packed_vec::PackedVec;

// http://pages.cs.wisc.edu/~jussara/papers/00ton.pdf
pub struct CountingBloom<K> {
    packed_vec: PackedVec,
    num_hashes: usize,
    _marker: PhantomData<K>,
}

impl<K: AsRef<[u8]>> CountingBloom<K> {
    pub fn new(num_items: u64, max_false_prob: f64) -> CountingBloom<K> {
        assert!(max_false_prob > 0.0 && max_false_prob < 1.0, "False positive probability must be in interval (0, 1)");
        let num_bits = Self::optimal_num_bits(num_items, max_false_prob);
        let num_hashes = Self::optimal_num_hashes(num_bits, num_items);
        CountingBloom {
            packed_vec: PackedVec::new(num_bits, 4),
            num_hashes: num_hashes,
            _marker: PhantomData,
        }
    }

    // https://corte.si/posts/code/bloom-filter-rules-of-thumb/index.html
    fn optimal_num_bits(num_items: u64, max_false_prob: f64) -> usize {
        let ln2_squared = 2_f64.ln() * 2_f64.ln();
        let numerator = num_items as f64 * (1_f64/max_false_prob).ln();
        (numerator / ln2_squared).round() as usize
    }

    fn optimal_num_hashes(num_bits: usize, num_items: u64) -> usize {
        let num_hashes = (num_bits as f64 * 2_f64.ln()) / num_items as f64;
        num_hashes.round() as usize
    }

    fn key_hashes(key: K) -> Hashes {
        let mut hash_result = [0u8; 16];
        let mut key_reader = Cursor::new(key);
        murmur3_x64_128(&mut key_reader, 0, &mut hash_result);

        let hash1 = BigEndian::read_u64(&hash_result);
        let hash2 = BigEndian::read_u64(&hash_result[4..]);

        Hashes::new(hash1, hash2)
    }

    pub fn add(&mut self, key: K) {
        for hash in Self::key_hashes(key).take(self.num_hashes) {
            let count = self.packed_vec.len();
            self.packed_vec.increment(hash as usize % count);
        }
    }

    pub fn remove(&mut self, key: K) {
        for hash in Self::key_hashes(key).take(self.num_hashes) {
            let count = self.packed_vec.len();
            self.packed_vec.decrement(hash as usize % count);
        }
    }

    pub fn contains(&mut self, key: K) -> bool {
        let mut contains_key = true;
        for hash in Self::key_hashes(key).take(self.num_hashes) {
            let count = self.packed_vec.len();
            contains_key &= self.packed_vec.get(hash as usize % count).unwrap() > 0;
        }
        contains_key
    }
}

#[cfg(test)]
mod tests {
    extern crate test;

    use self::test::{Bencher, black_box};

    use super::CountingBloom;
    use hamcrest::*;

    #[test]
    fn added_value_is_part_of_a_set() {
        let mut bloom: CountingBloom<&'static str> = CountingBloom::new(10_000, 0.01);
        bloom.add("a");
        assert_that(bloom.contains("a"), is(equal_to(true)));
        assert_that(bloom.contains("b"), is(equal_to(false)));
    }

    #[bench]
    fn creation_overhead(b: &mut Bencher) {
        b.iter(|| {
            let bloom: CountingBloom<&'static str> = CountingBloom::new(10_000, 0.03);
            black_box(bloom)
        })
    }

    #[bench]
    fn add_element(b: &mut Bencher) {
        let mut bloom: CountingBloom<&'static str> = CountingBloom::new(10_000, 0.03);
        b.iter(|| {
            bloom.add("a");
            bloom.remove("a")
        })
    }

    #[bench]
    fn check_contains_element(b: &mut Bencher) {
        let mut bloom: CountingBloom<&'static str> = CountingBloom::new(10_000, 0.03);
        bloom.add("a");
        b.iter(|| {
            black_box(bloom.contains("a"))
        })
    }
}
