const BLOCK_COUNT: usize = 24;
const START_SIZE: usize = 8;
const LOG2_OF_START_SIZE: usize = 3;

/// SegmentedIdx represents a cached index lookup into the segmented list, computed with
/// `SegmentedList::compute_segmented_idx`, can be used with `SegmentedList::get_with_segmented_idx`
/// and `SegmentedList::get_mut_with_segmented_idx`.
///
/// Primary usecase is to cache the lookup of many idxes, thus omiting the lookup computation which
/// can be too heavy in intensive workloads.
#[derive(Copy, Clone)]
pub struct SegmentedIdx(usize, usize);

/// SegmentedList is a drop in `std::vec::Vec` replacement providing zero cost growing and stable
/// pointers even after grow with `::push`.
///
/// The list is implemented by chaining blocks of memory to store its elements. Each block is
/// allocated on demand when an index falls into it (for instance during appends), starting at
/// `START_SIZE` elements in the first block and doubling the block size for each subsequent
/// allocation. This continues until `BLOCK_COUNT` is reached. Existing blocks are never moved or
/// reallocated, so references into the list remain valid across growth operations.
///
/// This makes the SegmentedList an adequate replacement for `std::vec::Vec` when dealing with
/// heavy and unpredictable growth workloads due the omission of copy/move overhead on expansion.
pub struct SegmentedList<T> {
    /// blocks grow by two on each newly allocated block, up until BLOCK_COUNT or something
    blocks: [Option<Box<[std::mem::MaybeUninit<T>]>>; BLOCK_COUNT],
    len: usize,
}

impl<T> SegmentedList<T> {
    fn idx_to_block_idx(&self, idx: usize) -> SegmentedIdx {
        // we are in the size of the first block, no computation necessary
        if idx < START_SIZE {
            return SegmentedIdx(0, idx);
        }

        let adjusted = idx + START_SIZE;
        let msb_pos: usize = 63 - adjusted.leading_zeros() as usize;

        let block = msb_pos - LOG2_OF_START_SIZE;
        let block_start = START_SIZE * ((1 << block) - 1);

        SegmentedIdx(block, idx - block_start)
    }

    fn alloc_block(&mut self, block: usize) {
        let size = START_SIZE << block;
        let mut v: Vec<std::mem::MaybeUninit<T>> = Vec::with_capacity(size);
        unsafe {
            v.set_len(size);
        }
        self.blocks[block] = Some(v.into_boxed_slice());
    }

    /// Create zero allocation empty list
    pub fn new() -> Self {
        Self {
            blocks: std::array::from_fn(|_| None),
            len: 0,
        }
    }

    /// Computes the SegmentedIdx for idx, block refers to the block inside of Self storing
    /// the value for the idx, block_idx is the index into said block
    pub fn compute_segmented_idx(&self, idx: usize) -> Option<SegmentedIdx> {
        if idx > self.len {
            None
        } else {
            Some(self.idx_to_block_idx(idx))
        }
    }

    /// Appends `v` at the end of `Self`
    pub fn push(&mut self, v: T) {
        let idx = self.len;
        let SegmentedIdx(block, block_index) = self.idx_to_block_idx(idx);
        if self.blocks[block].is_none() {
            self.alloc_block(block);
        }

        self.blocks[block].as_mut().unwrap()[block_index].write(v);
        self.len += 1;
    }

    /// Uses precomputed `SegmentedIdx` to return a reference to the element at `idx`
    pub fn get_with_segmented_idx(&self, idx: SegmentedIdx) -> Option<&T> {
        let SegmentedIdx(block, block_index) = idx;
        self.blocks[block]
            .as_ref()
            .map(|b| unsafe { b[block_index].assume_init_ref() })
    }

    /// Uses precomputed `SegmentedIdx` to return a mutable reference to the element at `idx`
    pub fn get_mut_with_segmented_idx(&mut self, idx: SegmentedIdx) -> Option<&mut T> {
        let SegmentedIdx(block, block_index) = idx;
        self.blocks[block]
            .as_mut()
            .map(|b| unsafe { b[block_index].assume_init_mut() })
    }

    /// Returns a reference to the element at `idx`
    pub fn get(&self, idx: usize) -> Option<&T> {
        if idx >= self.len {
            return None;
        }
        let SegmentedIdx(block, block_index) = self.idx_to_block_idx(idx);
        self.blocks[block]
            .as_ref()
            .map(|b| unsafe { b[block_index].assume_init_ref() })
    }

    /// Returns a mutable reference to the element at `idx`
    pub fn get_mut(&mut self, idx: usize) -> Option<&mut T> {
        if idx >= self.len {
            return None;
        }
        let SegmentedIdx(block, block_index) = self.idx_to_block_idx(idx);
        self.blocks[block]
            .as_mut()
            .map(|b| unsafe { b[block_index].assume_init_mut() })
    }

    /// Returns the length of self
    pub fn len(&self) -> usize {
        self.len
    }

    /// Collects self and its contents into a vec
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

    pub fn capacity(&self) -> usize {
        let mut total = 0;
        for block_idx in 0..BLOCK_COUNT {
            let Some(block) = &self.blocks[block_idx] else {
                break;
            };
            total += block.len()
        }
        total
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn first(&self) -> Option<&T> {
        if self.len == 0 {
            None
        } else {
            // first element is always at idx 0 of block 0, thus we hardcode this
            self.get_with_segmented_idx(SegmentedIdx(0, 0))
        }
    }

    pub fn first_mut(&mut self) -> Option<&mut T> {
        if self.len == 0 {
            None
        } else {
            self.get_mut_with_segmented_idx(SegmentedIdx(0, 0))
        }
    }

    pub fn last(&self) -> Option<&T> {
        if self.len == 0 {
            None
        } else {
            let si = self.idx_to_block_idx(self.len - 1);
            self.get_with_segmented_idx(si)
        }
    }

    pub fn last_mut(&mut self) -> Option<&mut T> {
        if self.len == 0 {
            None
        } else {
            let si = self.idx_to_block_idx(self.len - 1);
            self.get_mut_with_segmented_idx(si)
        }
    }

    pub fn clear(&mut self) {
        let mut remaining = self.len;
        for block_idx in 0..BLOCK_COUNT {
            if remaining == 0 {
                break;
            }
            let Some(block) = &mut self.blocks[block_idx] else {
                break;
            };
            let block_size = START_SIZE << block_idx;
            let take = remaining.min(block_size);
            for i in 0..take {
                unsafe { block[i].assume_init_drop() };
            }
            remaining -= take;
        }
        self.len = 0;
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

        let SegmentedIdx(block, block_index) = self.idx_to_block_idx(idx);
        let block_ref = self.blocks[block].as_ref().unwrap();
        unsafe { block_ref[block_index].assume_init_ref() }
    }
}

impl<T> std::ops::IndexMut<usize> for SegmentedList<T> {
    fn index_mut(&mut self, idx: usize) -> &mut T {
        if idx >= self.len {
            panic!(
                "index {} out of bounds for List of length {}",
                idx, self.len
            );
        }

        let SegmentedIdx(block, block_index) = self.idx_to_block_idx(idx);
        let block_ref = self.blocks[block].as_mut().unwrap();
        unsafe { block_ref[block_index].assume_init_mut() }
    }
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

impl<T> Extend<T> for SegmentedList<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for item in iter {
            self.push(item);
        }
    }
}

impl<T> std::iter::FromIterator<T> for SegmentedList<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut sl = SegmentedList::new();
        sl.extend(iter);
        sl
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, rc::Rc};

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
    fn push_and_get_mut() {
        let mut list = SegmentedList::new();

        list.push(42);
        list.push(100);
        list.push(7);

        assert_eq!(list.len, 3);
        assert_eq!(list.get(0), Some(&42));
        assert_eq!(list.get(1), Some(&100));
        assert_eq!(list.get(2), Some(&7));
        assert_eq!(list.get(3), None);
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
    fn index_mut_trait_returns_correct_values() {
        let mut list = SegmentedList::new();
        for i in 0..10 {
            list.push(i * 2);
        }

        assert_eq!(list[0], 0);
        assert_eq!(list[1], 2);
        assert_eq!(list[9], 18);

        list[0] = 5;
        list[1] = 3;
        list[9] = 36;

        assert_eq!(list[0], 5);
        assert_eq!(list[1], 3);
        assert_eq!(list[9], 36);
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
        let blocks_to_test = 3;
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

    #[test]
    fn capacity_and_is_empty_work() {
        let mut list = SegmentedList::new();
        assert_eq!(list.capacity(), 0);
        assert!(list.is_empty());

        list.push(10);
        assert!(!list.is_empty());
        assert!(list.capacity() >= 1);
    }

    #[test]
    fn first_and_last_work() {
        let mut list = SegmentedList::new();
        assert!(list.first().is_none());
        assert!(list.last().is_none());

        list.push(1);
        list.push(2);
        list.push(3);
        assert_eq!(list.len(), 3);

        assert_eq!(list.first(), Some(&1));
        assert_eq!(list.last(), Some(&3));

        *list.first_mut().unwrap() = 10;
        *list.last_mut().unwrap() = 30;

        assert_eq!(list.first(), Some(&10));
        assert_eq!(list[1], 2);
        assert_eq!(list.last(), Some(&30));
    }

    #[test]
    fn clear_resets_len_and_drops_items() {
        struct DropCounter<'a>(&'a RefCell<u32>);
        impl<'a> Drop for DropCounter<'a> {
            fn drop(&mut self) {
                *self.0.borrow_mut() += 1;
            }
        }
        let counter = Rc::new(RefCell::new(0));
        let mut list: SegmentedList<DropCounter> = SegmentedList::new();

        for _ in 0..10 {
            list.push(DropCounter(&counter));
        }
        assert_eq!(list.len(), 10);
        list.clear();
        assert_eq!(list.len(), 0);
        assert_eq!(*counter.borrow(), 10);
        assert!(list.is_empty());
    }

    #[test]
    fn extend_trait_adds_items() {
        let mut list = SegmentedList::new();
        list.extend(vec![1, 2, 3]);
        assert_eq!(list.len(), 3);
        assert_eq!(list.to_vec(), vec![1, 2, 3]);
    }

    #[test]
    fn from_iterator_constructs_list() {
        let list: SegmentedList<_> = (0..5).collect();
        assert_eq!(list.len(), 5);
        assert_eq!(list.to_vec(), (0..5).collect::<Vec<_>>());
    }
}
