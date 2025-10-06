# Segmented-rs

Segmented-rs is a (zero dependency) rusty port of a segmented list and bump
allocator that was initially implemented in c, it's usage can be summarized as
follows:

```rust
use segmented_rs::{alloc, list::SegmentedList};

fn main() {
    let mut list = SegmentedList::new();
    let count = 8 * 1000 * 1000;
    for i in 0..count {
        list.push(i);
    }
    assert_eq!(list.len(), count);

    let cloned_list = list.clone();
    // std::vec::Vec interop
    let as_vec = cloned_list.to_vec();
    assert_eq!(list.len(), as_vec.len());
    assert_eq!(list[count / 2], as_vec[count / 2]);

    // caching indizes for 0 cost indexing
    let idx = (count as f64 / 3.1415) as usize;
    let cached_idx = list
        .compute_segmented_idx(idx) // perfoms indexing logic taking list len into account
        .expect("Idx out of list bounds");

    assert_eq!(
        // omits bounds checks and indexing logic
        list.get_with_segmented_idx(cached_idx),
        // does bounds checks and indexing logic
        list.get(idx),
    );
}
```

## Features:

- zero dependencies
- fully tested
- `alloc::SegmentedAlloc`: 
    - not thread safe allocator specifically for `list::SegmentedList`
    - segmented bump allocator backed by mmap
    - no drop, no dealloc
- `list::SegmentedList<T>`:
    - no copy, bump allocator backed dynamic array
    - heavier indexing but extremly cheap grows without moving or copying memory
    - caching of indexing is available via `list::SegmentedIdx`,
      `SegmentedList::compute_segmented_idx`,
      `SegmentedList::get_with_segmented_idx` and
      `SegmentedList::get_mut_with_segmented_idx`
- `mmap::mmap` and `mmap::munmap`:
    - x86 based handrolled wrapper 
    - wrapping syscalls with `NonNull`
    - mapping arguments to rust type system via `MmapFlags` and `MmapProt`
