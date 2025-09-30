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
