# Segmented-rs

Segmented-rs is a rusty port of a segmented list and bump allocator that was
initially implemented in c, it's usage can be summarized as follows:

```rust
use segmented_rs::{alloc, list::SegmentedList};

#[global_allocator]
static A: alloc::SegmentedAlloc = alloc::SegmentedAlloc::new();

fn main() {
    let mut list = SegmentedList::new();
    let count = 8 * 1000 * 1000;
    for i in 0..count {
        list.push(i);
    }

    assert_eq!(list.len(), count);

    let cloned_list = list.clone();
    let as_vec = cloned_list.to_vec();
    assert_eq!(list.len(), as_vec.len());
    assert_eq!(list[count / 2], as_vec[count / 2]);
}
```

## Features:

- zero dependencies
- `alloc::SegmentedAlloc`: 
    - global not thread safe allocator
    - segmented bump allocator backed by mmap
    - no drop, no dealloc
- `list::SegmentedList<T>`:
    - no copy, bump allocator backed dynamic array
    - heavier inserts and lookups, but very cheap grows
- `mmap::mmap` and `mmap::munmap`:
    - x86 based handrolled wrapper 
    - wrapping syscalls with `NonNull`
    - mapping arguments to rust type system via `MmapFlags` and `MmapProt`
