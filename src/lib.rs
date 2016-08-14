#![cfg_attr(test, feature(test))]

extern crate bit_vec;
extern crate murmur3;
extern crate byteorder;

#[cfg(test)]
extern crate hamcrest;

mod hashes;

use bit_vec::BitVec;
use murmur3::murmur3_x64_128;
use std::marker::PhantomData;
use std::io::Cursor;
use byteorder::{BigEndian, ByteOrder};

use hashes::Hashes;

// http://dmod.eu/deca/ft_gateway.cfm.pdf
pub struct Bloom<K> {
    bit_vec: BitVec,
    num_hashes: usize,
    _marker: PhantomData<K>,
}

impl<K: AsRef<[u8]>> Bloom<K> {
    pub fn new(num_items: u64, max_false_prob: f64) -> Bloom<K> {
        assert!(max_false_prob > 0.0 && max_false_prob < 1.0, "False positive probability must be in interval (0, 1)");
        let num_bits = Self::optimal_num_bits(num_items, max_false_prob);
        let num_hashes = Self::optimal_num_hashes(num_bits, num_items);
        Bloom {
            bit_vec: BitVec::from_elem(num_bits, false),
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
            let count = self.bit_vec.len();
            self.bit_vec.set(hash as usize % count, true);
        }
    }

    pub fn contains(&self, key: K) -> bool {
        let mut contains_key = true;
        for hash in Self::key_hashes(key).take(self.num_hashes) {
            let count = self.bit_vec.len();
            contains_key &= self.bit_vec.get(hash as usize % count).unwrap();
        }
        contains_key
    }
}

#[cfg(test)]
mod tests {
    extern crate test;

    use self::test::{Bencher, black_box};

    use super::Bloom;
    use super::hamcrest::*;

    #[test]
    fn added_value_is_part_of_a_set() {
        let mut bloom: Bloom<&'static str> = Bloom::new(10_000, 0.01);
        bloom.add("a");
        assert_that(bloom.contains("a"), is(equal_to(true)));
        assert_that(bloom.contains("b"), is(equal_to(false)));
    }

    #[bench]
    fn creation_overhead(b: &mut Bencher) {
        b.iter(|| {
            let bloom: Bloom<&'static str> = Bloom::new(10_000, 0.03);
            black_box(bloom)
        })
    }

    #[bench]
    fn add_element(b: &mut Bencher) {
        let mut bloom: Bloom<&'static str> = Bloom::new(10_000, 0.03);
        b.iter(|| {
            bloom.add("a")
        })
    }

    #[bench]
    fn check_contains_element(b: &mut Bencher) {
        let mut bloom: Bloom<&'static str> = Bloom::new(10_000, 0.03);
        bloom.add("a");
        b.iter(|| {
            black_box(bloom.contains("a"))
        })
    }
}
