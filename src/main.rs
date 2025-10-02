use segmented_rs::list::SegmentedList;

// #[global_allocator]
// static A: alloc::SegmentedAlloc = alloc::SegmentedAlloc::new();

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
