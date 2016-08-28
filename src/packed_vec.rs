use std::mem::size_of;
use std::usize;
use std::cmp::{max, min};

pub struct PackedVec {
    num_elem: usize,
    element_size_bits: usize,
    bucket_size_bits: usize,
    elements_per_bucket: usize,
    data: Vec<usize>,
}

impl PackedVec {
    pub fn new(num_elem: usize, element_size_bits: usize) -> PackedVec {
        let bucket_size_bits = size_of::<usize>() * 8;
        let elements_per_bucket = (bucket_size_bits as f64 / element_size_bits as f64).floor();
        let vec_size = (num_elem as f64 / elements_per_bucket).ceil() as usize;
        PackedVec {
            num_elem: num_elem,
            element_size_bits: element_size_bits,
            bucket_size_bits: bucket_size_bits,
            elements_per_bucket: elements_per_bucket as usize,
            data: vec![0; vec_size],
        }
    }

    pub fn len(&self) -> usize {
        self.num_elem
    }

    fn with_element<F>(&mut self, index: usize, mut f: F)
        where F: FnMut(usize) -> Option<usize>
    {
        let bucket_index = index / self.elements_per_bucket;
        let bucket = self.data[bucket_index];
        let element_index = index % self.elements_per_bucket;
        let element_mask = if self.element_size_bits == size_of::<usize>() * 8 {
            usize::MAX
        } else {
            (1 << self.element_size_bits) - 1
        };
        let shift_size = self.bucket_size_bits - self.element_size_bits * (element_index + 1);
        let element = bucket >> shift_size & element_mask;
        match f(element) {
            Some(new_element) => {
                let new_value = Self::cap_value_to_valid_range(new_element, element_mask);
                let clear_mask = !(element_mask << shift_size);
                let cleared_bucket = bucket & clear_mask;
                let new_bucket = cleared_bucket | (new_value << shift_size);
                self.data[bucket_index] = new_bucket;
            }
            None => {}
        }
    }

    fn cap_value_to_valid_range(value: usize, max_value: usize) -> usize {
        min(max(value, 0), max_value)
    }

    pub fn increment(&mut self, index: usize) {
        self.with_element(index, |element| {
            Some(element + 1)
        });
    }

    pub fn decrement(&mut self, index: usize) {
        self.with_element(index, |element| {
            if element == 0 {
                Some(0)
            } else {
                Some(element - 1)
            }
        });
    }

    pub fn get(&mut self, index: usize) -> Option<usize> {
        let mut found_element = None;
        self.with_element(index, |element| {
            found_element = Some(element);
            None
        });
        found_element
    }

    #[allow(dead_code)]
    fn set(&mut self, index: usize, value: usize) {
        self.with_element(index, |_element| {
            Some(value)
        });
    }
}

#[cfg(test)]
mod tests {
    extern crate test;

    use self::test::Bencher;

    use super::*;
    use hamcrest::*;
    use rand::{self, Rng};

    fn new_default() -> PackedVec {
        PackedVec::new(20, 4)
    }

    #[test]
    fn value_at_index_is_incremented() {
        let mut v = new_default();
        assert_that(v.get(0), is(equal_to(Some(0))));
        v.increment(0);
        assert_that(v.get(0), is(equal_to(Some(1))));
        v.increment(0);
        v.increment(1);
        assert_that(v.get(0), is(equal_to(Some(2))));
        assert_that(v.get(1), is(equal_to(Some(1))));
    }

    #[test]
    fn value_when_incrementing_is_capped_at_max_valid_value() {
        let mut v = PackedVec::new(5, 2);
        v.set(0, 2);
        v.increment(0);
        assert_that(v.get(0), is(equal_to(Some(3))));
        v.increment(0);
        assert_that(v.get(0), is(equal_to(Some(3))));
    }

    #[test]
    fn value_when_decrementing_is_capped_at_zero() {
        let mut v = PackedVec::new(5, 2);
        v.set(0, 1);
        v.decrement(0);
        assert_that(v.get(0), is(equal_to(Some(0))));
        v.decrement(0);
        assert_that(v.get(0), is(equal_to(Some(0))));
    }

    #[test]
    fn value_when_setting_is_capped_to_max_valid_value() {
        let mut v = PackedVec::new(5, 2);
        v.set(0, 100);
        assert_that(v.get(0), is(equal_to(Some(3))));
    }

    #[test]
    fn value_at_index_is_decremented() {
        let mut v = new_default();
        v.set(0, 3);
        v.set(1, 1);
        v.decrement(0);
        assert_that(v.get(0), is(equal_to(Some(2))));
        v.decrement(0);
        v.decrement(0);
        v.decrement(1);
        assert_that(v.get(0), is(equal_to(Some(0))));
        assert_that(v.get(1), is(equal_to(Some(0))));
    }

    #[test]
    fn all_values_can_be_decremented_and_incremented() {
        let mut v = new_default();
        for i in 0..v.len() {
            assert_that(v.get(i), is(equal_to(Some(0))));
            v.increment(i);
            assert_that(v.get(i), is(equal_to(Some(1))));
            v.decrement(i);
            assert_that(v.get(i), is(equal_to(Some(0))));
        };
    }

    #[bench]
    fn increment_and_decrement_random(b: &mut Bencher) {
        let mut v = PackedVec::new(5000, 2);
        let mut rng = rand::thread_rng();
        let indexes: Vec<_> = rng.gen_iter::<usize>().map(|x| x % v.len()).take(1000).collect();
        b.iter(|| {
            for i in indexes.iter() {
                v.increment(*i);
                v.decrement(*i);
            }
        })
    }

    #[bench]
    fn increment_and_decrement_sequential(b: &mut Bencher) {
        let mut v = PackedVec::new(5000, 2);
        let indexes: Vec<_> = (0..v.len()).cycle().take(1000).collect();
        b.iter(|| {
            for i in indexes.iter() {
                v.increment(*i);
                v.decrement(*i);
            }
        })
    }
}
