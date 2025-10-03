use std::alloc::GlobalAlloc;
use std::cell::UnsafeCell;
use std::fmt::Display;
use std::ptr::NonNull;

use crate::mmap::{self, mmap};

const MIN_SIZE: usize = 4096;
const MAX_BLOCKS: usize = 55;
const GROWTH: usize = 2;

#[derive(Debug)]
struct SegmentedAllocCtx {
    /// idx into self.blocks
    cur_block: usize,
    /// size of the current block
    size: usize,
    /// bytes in use of the current block
    pos: usize,
    blocks: [Option<NonNull<u8>>; MAX_BLOCKS],
    block_sizes: [usize; MAX_BLOCKS],
}

impl SegmentedAllocCtx {
    const fn new() -> Self {
        SegmentedAllocCtx {
            size: MIN_SIZE,
            cur_block: 0,
            pos: 0,
            blocks: [const { None }; MAX_BLOCKS],
            block_sizes: [0; MAX_BLOCKS],
        }
    }
}

/// Implements a variable size bump allocator, employing mmap to allocate a starting block of
/// 4096B, once a block is exceeded by a request, the allocator mmaps a new block double the size
/// of the previously allocated block
pub struct SegmentedAlloc {
    ctx: UnsafeCell<SegmentedAllocCtx>,
}

impl Display for SegmentedAlloc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner_ref = unsafe { &*self.ctx.get() };
        write!(f, "{:?}", inner_ref)
    }
}

unsafe impl Send for SegmentedAlloc {}
unsafe impl Sync for SegmentedAlloc {}

impl SegmentedAlloc {
    pub const fn new() -> Self {
        Self {
            ctx: UnsafeCell::new(SegmentedAllocCtx::new()),
        }
    }

    fn request(&self, layout: std::alloc::Layout) -> NonNull<u8> {
        let ctx = unsafe { &mut *self.ctx.get() };
        if ctx.blocks[0].is_none() {
            ctx.block_sizes[0] = MIN_SIZE;
            let block = mmap(
                None,
                MIN_SIZE,
                mmap::MmapProt::READ | mmap::MmapProt::WRITE,
                mmap::MmapFlags::PRIVATE | mmap::MmapFlags::ANONYMOUS,
                -1,
                0,
            );
            ctx.blocks[0] = Some(block);
        }

        loop {
            let offset = ctx.pos.next_multiple_of(layout.align());
            let padded_size = layout.pad_to_align().size();

            let end = offset
                .checked_add(padded_size)
                .expect("Allocation size overflow");

            if end > ctx.size {
                assert!(ctx.cur_block + 1 < MAX_BLOCKS, "Exceeded MAX_BLOCKS");
                let new_size = ctx.size * GROWTH;
                let block = mmap(
                    None,
                    new_size,
                    mmap::MmapProt::READ | mmap::MmapProt::WRITE,
                    mmap::MmapFlags::PRIVATE | mmap::MmapFlags::ANONYMOUS,
                    -1,
                    0,
                );
                ctx.cur_block += 1;
                ctx.blocks[ctx.cur_block] = Some(block);
                ctx.block_sizes[ctx.cur_block] = new_size;
                ctx.size = new_size;
                ctx.pos = 0;
                continue;
            }

            let block = ctx.blocks[ctx.cur_block].expect("Current block is None while allocating");

            let ptr = unsafe { block.as_ptr().add(offset) };
            ctx.pos = end;
            return NonNull::new(ptr).expect("failed to create nonnull");
        }
    }
}

/*
impl Drop for SegmentedAlloc {
    fn drop(&mut self) {
        let ctx = unsafe { &mut *self.ctx.get() };
        for i in 0..MAX_BLOCKS {
            let size = ctx.block_sizes[i];
            if size == 0 {
                break;
            }

            let Some(block) = ctx.blocks[i] else {
                break;
            };
            munmap(block, size);
        }
    }
}
*/

// TODO: when used as global alloc results in segfaults, i have no clue why, but it seems to touch
// memory that is no longer valid; but only in release mode (-C debug-assertions removes the
// fault):
//
// fish: Job 1, 'cargo run --release' terminated by signal SIGSEGV (Address boundary error)
//
// Program received signal SIGSEGV, Segmentation fault.
// 0x0000555555569c7b in <segmented_rs::alloc::SegmentedAlloc as core::alloc::global::GlobalAlloc>::alloc ()
// (gdb) bt
// #0  0x0000555555569c7b in <segmented_rs::alloc::SegmentedAlloc as core::alloc::global::GlobalAlloc>::alloc ()
// #1  0x0000555555585f11 in std::rt::lang_start_internal ()
// #2  0x0000555555569bf5 in main ()

unsafe impl GlobalAlloc for SegmentedAlloc {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        #[cfg(feature = "trace")]
        eprintln!(
            "[SegmentedAlloc] alloc size={}, align={}",
            layout.size(),
            layout.align()
        );
        self.request(layout).as_ptr()
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: std::alloc::Layout) {
        #[cfg(feature = "trace")]
        eprintln!(
            "[SegmentedAlloc] dealloc size={}, align={}",
            _layout.size(),
            _layout.align()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::alloc::{GlobalAlloc, Layout};

    #[test]
    fn alloc_min_size() {
        let alloc = SegmentedAlloc::new();
        unsafe {
            let layout = Layout::from_size_align(8, 8).unwrap();
            let ptr1 = alloc.alloc(layout);
            assert!(!ptr1.is_null());
            assert_eq!(ptr1 as usize % layout.align(), 0);
        }
    }

    #[test]
    fn alloc_multiple_blocks() {
        let alloc = SegmentedAlloc::new();
        unsafe {
            let layout = Layout::from_size_align(4096, 8).unwrap();
            let ptr1 = alloc.alloc(layout);
            assert!(!ptr1.is_null());
            for _ in 0..(MIN_SIZE / 8) {
                alloc.alloc(Layout::from_size_align(8, 8).unwrap());
            }
            let ptr2 = alloc.alloc(Layout::from_size_align(8, 8).unwrap());
            assert!(!ptr2.is_null());
        }
    }

    #[test]
    fn allocations_do_not_overlap() {
        let alloc = SegmentedAlloc::new();
        unsafe {
            let layout = Layout::from_size_align(16, 8).unwrap();
            let p1 = alloc.alloc(layout);
            let p2 = alloc.alloc(layout);
            assert!(p1 != p2);
        }
    }

    #[test]
    fn allocation_alignment_respected() {
        let alloc = SegmentedAlloc::new();
        unsafe {
            let layout = Layout::from_size_align(32, 32).unwrap();
            let p = alloc.alloc(layout);
            assert_eq!(p as usize % 32, 0);
        }
    }

    #[test]
    fn stress_many_allocations() {
        let alloc = SegmentedAlloc::new();
        unsafe {
            for size in [8usize, 16, 64, 128, 256, 1024, 2048] {
                let layout = Layout::from_size_align(size, 8).unwrap();
                for _ in 0..1000 {
                    let _ = alloc.alloc(layout);
                }
            }
        }
    }

    #[test]
    fn allocate_a_gigabyte() {
        use std::alloc::Layout;

        let alloc = SegmentedAlloc::new();
        let gig: usize = 1024 * 1024 * 1024;
        let chunk: usize = 4096;
        let layout = Layout::from_size_align(chunk, 8).unwrap();
        let chunks = gig / chunk;

        // TOUCHING :^) the whole gig so the kernel will allocate each byte
        unsafe {
            for i in 0..chunks {
                let ptr = alloc.alloc(layout);
                assert!(!ptr.is_null());
                // Touch first byte so OS actually backs the page
                std::ptr::write_bytes(ptr, (i % 255) as u8, 1);
            }
        }
    }

    #[test]
    fn allocate_ten_gigabyte() {
        use std::alloc::Layout;

        let alloc = SegmentedAlloc::new();
        let gig: usize = 10 * 1024 * 1024 * 1024;
        let chunk: usize = 4096;
        let layout = Layout::from_size_align(chunk, 8).unwrap();
        let chunks = gig / chunk;

        // TOUCHING :^) the whole ten gig so the kernel will allocate each byte
        unsafe {
            for i in 0..chunks {
                let ptr = alloc.alloc(layout);
                assert!(!ptr.is_null());
                // Touch first byte so OS actually backs the page
                std::ptr::write_bytes(ptr, (i % 255) as u8, 1);
            }
        }
    }
}
