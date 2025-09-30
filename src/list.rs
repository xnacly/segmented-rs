const BLOCK_COUNT: usize = 24;
const START_SIZE: usize = 8;
const LOG2_OF_START_SIZE: usize = 3;

pub struct SegmentedList<T> {
    /// blocks grow by two on each newly allocated block, up until BLOCK_COUNT or something
    blocks: [Option<Box<[std::mem::MaybeUninit<T>]>>; BLOCK_COUNT],
    len: usize,
}

impl<T: Clone + Copy> Clone for SegmentedList<T> {
    fn clone(&self) -> Self {
        let mut new_blocks: [Option<Box<[std::mem::MaybeUninit<T>]>>; BLOCK_COUNT] =
            Default::default();
        for i in 0..BLOCK_COUNT {
            new_blocks[i] = self.blocks[i].as_ref().map(|b| b.clone());
        }
        Self {
            blocks: new_blocks,
            len: self.len.clone(),
        }
    }
}

impl<T> SegmentedList<T> {
    fn idx_to_block_idx(&self, idx: usize) -> (usize, usize) {
        // we are in the size of the first block, no computation necessary
        if idx < START_SIZE {
            return (0, idx);
        }

        let adjusted = idx + START_SIZE;
        let msb_pos: usize = 63 - adjusted.leading_zeros() as usize;

        let block = msb_pos - LOG2_OF_START_SIZE;
        let block_start = START_SIZE * ((1 << block) - 1);

        (block, idx - block_start)
    }

    fn alloc_block(&mut self, block: usize) {
        let size = START_SIZE << block;
        let mut v: Vec<std::mem::MaybeUninit<T>> = Vec::with_capacity(size);
        unsafe {
            v.set_len(size);
        }
        self.blocks[block] = Some(v.into_boxed_slice());
    }

    pub fn new() -> Self {
        Self {
            blocks: std::array::from_fn(|_| None),
            len: 0,
        }
    }

    pub fn push(&mut self, v: T) {
        let idx = self.len;
        let (block, block_index) = self.idx_to_block_idx(idx);
        if self.blocks[block].is_none() {
            self.alloc_block(block);
        }

        self.blocks[block].as_mut().unwrap()[block_index].write(v);
        self.len += 1;
    }

    pub fn get(&self, idx: usize) -> Option<&T> {
        if idx >= self.len {
            return None;
        }
        let (block, block_index) = self.idx_to_block_idx(idx);
        self.blocks[block]
            .as_ref()
            .map(|b| unsafe { b[block_index].assume_init_ref() })
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn to_vec(mut self) -> Vec<T> {
        let mut result = Vec::with_capacity(self.len);
        let mut remaining = self.len;

        for block_idx in 0..BLOCK_COUNT {
            if remaining == 0 {
                break;
            }

            if let Some(block) = self.blocks[block_idx].take() {
                let block_size = START_SIZE << block_idx;
                let take = remaining.min(block_size);

                for i in 0..take {
                    let value = unsafe { block[i].assume_init_read() };
                    result.push(value);
                }

                remaining -= take;
            } else {
                break;
            }
        }

        result
    }
}

impl<T> std::default::Default for SegmentedList<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> std::ops::Index<usize> for SegmentedList<T> {
    type Output = T;

    fn index(&self, idx: usize) -> &Self::Output {
        if idx >= self.len {
            panic!(
                "index {} out of bounds for List of length {}",
                idx, self.len
            );
        }

        let (block, block_index) = self.idx_to_block_idx(idx);
        let block_ref = self.blocks[block].as_ref().unwrap();
        unsafe { block_ref[block_index].assume_init_ref() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alloc;
    use std::{cell::RefCell, rc::Rc};

    #[global_allocator]
    static A: alloc::SegmentedAlloc = alloc::SegmentedAlloc::new();

    #[test]
    fn push_and_get_basic() {
        let mut list = SegmentedList::new();

        list.push(42);
        list.push(100);
        list.push(7);

        assert_eq!(list.len, 3);
        assert_eq!(list.get(0), Some(&42));
        assert_eq!(list.get(1), Some(&100));
        assert_eq!(list.get(2), Some(&7));
        assert_eq!(list.get(3), None); // out of bounds
    }

    #[test]
    fn into_vec_flattens_correctly() {
        let mut list = SegmentedList::new();

        for i in 0..20 {
            list.push(i);
        }

        let vec = list.to_vec();
        assert_eq!(vec.len(), 20);
        assert_eq!(vec, (0..20).collect::<Vec<_>>());
    }

    #[test]
    fn index_trait_returns_correct_values() {
        let mut list = SegmentedList::new();
        for i in 0..10 {
            list.push(i * 2);
        }
        assert_eq!(list[0], 0);
        assert_eq!(list[1], 2);
        assert_eq!(list[9], 18);
    }

    #[test]
    #[should_panic(expected = "out of bounds")]
    fn index_panics_on_invalid() {
        let mut list = SegmentedList::new();
        list.push(1);
        let _ = list[1]; // index 1 invalid (len = 1)
    }

    #[test]
    fn works_across_blocks() {
        let mut list = SegmentedList::new();

        // Fill more than START_SIZE to force allocation of next block(s)
        for i in 0..(START_SIZE + 5) {
            list.push(i);
        }

        let vec = list.to_vec();
        assert_eq!(vec, (0..(START_SIZE + 5)).collect::<Vec<_>>());
    }

    #[test]
    fn empty_list_into_vec() {
        let list: SegmentedList<i32> = SegmentedList::new();
        let vec = list.to_vec();
        assert!(vec.is_empty());
    }

    #[test]
    fn exact_block_boundaries() {
        let mut list = SegmentedList::new();
        let blocks_to_test = 3; // test first few blocks
        let mut total = 0;
        for block_idx in 0..blocks_to_test {
            let size = START_SIZE << block_idx;
            for i in 0..size {
                list.push(total + i);
            }
            total += size;
            assert_eq!(list.len, total);
        }
        let vec = list.to_vec();
        assert_eq!(vec, (0..total).collect::<Vec<_>>());
    }

    #[test]
    fn drop_safety_test() {
        // Counter for tracking drops
        struct DropCounter<'a>(&'a RefCell<u32>);
        impl<'a> Drop for DropCounter<'a> {
            fn drop(&mut self) {
                *self.0.borrow_mut() += 1;
            }
        }

        let counter = Rc::new(RefCell::new(0));
        {
            let mut list: SegmentedList<DropCounter> = SegmentedList::new();
            for _ in 0..50 {
                list.push(DropCounter(&counter));
            }
            // consuming the list should drop all elements exactly once
            list.to_vec();
        }
        assert_eq!(*counter.borrow(), 50);
    }

    #[test]
    fn random_values_across_blocks() {
        let mut list = SegmentedList::new();
        // Push sparse and varied values
        for i in (0..(START_SIZE * 5)).rev() {
            // reverse order for variety
            list.push(i * 3);
        }
        let vec = list.to_vec();
        for (idx, val) in vec.iter().enumerate() {
            assert_eq!(*val, (START_SIZE * 5 - 1 - idx) * 3);
        }
    }

    #[test]
    fn stress_test_large_fill() {
        let mut list = SegmentedList::new();
        let count = START_SIZE * 50; // large, spans many blocks
        for i in 0..count {
            list.push(i);
        }
        let vec = list.to_vec();
        assert_eq!(vec.len(), count);
        assert_eq!(vec[count - 1], count - 1);
        assert_eq!(vec[0], 0);
    }
}
