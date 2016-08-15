use std::mem::size_of;

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
        let element_mask = (1 << self.element_size_bits) - 1; // handle 32
        let shift_size = self.bucket_size_bits - self.element_size_bits * (element_index + 1);
        let element = bucket >> shift_size & element_mask;
        match f(element) {
            Some(new_element) => {
                let clear_mask = !(element_mask << shift_size);
                let cleared_bucket = bucket & clear_mask;
                let new_bucket = cleared_bucket | (new_element << shift_size);
                self.data[bucket_index] = new_bucket;
            }
            None => {}
        }
    }

    pub fn increment(&mut self, index: usize) {
        self.with_element(index, |element| {
            Some(element + 1)
        });
    }

    pub fn decrement(&mut self, index: usize) {
        self.with_element(index, |element| {
            Some(element - 1)
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

    use super::PackedVec;
    use hamcrest::*;
    use rand::{self, Rng};

    #[test]
    fn value_at_index_is_incremented() {
        let mut v = PackedVec::new(20, 4);
        assert_that(v.get(0), is(equal_to(Some(0))));
        v.increment(0);
        assert_that(v.get(0), is(equal_to(Some(1))));
        v.increment(0);
        v.increment(1);
        assert_that(v.get(0), is(equal_to(Some(2))));
        assert_that(v.get(1), is(equal_to(Some(1))));
    }

    #[test]
    fn value_at_index_is_decremented() {
        let mut v = PackedVec::new(20, 4);
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

    #[bench]
    fn increment_and_decrement(b: &mut Bencher) {
        let mut v = PackedVec::new(20, 4);
        let mut rng = rand::thread_rng();
        b.iter(|| {
            let index = rng.gen::<usize>() % v.len();
            v.increment(index);
            v.decrement(index);
        })
    }
}
