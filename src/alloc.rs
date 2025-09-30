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
    curblock: usize,
    /// size of the current block
    size: usize,
    /// bytes in use of the current block
    pos: usize,
    blocks: [Option<NonNull<u8>>; MAX_BLOCKS],
    block_sizes: [Option<usize>; MAX_BLOCKS],
}

impl SegmentedAllocCtx {
    const fn new() -> Self {
        SegmentedAllocCtx {
            size: MIN_SIZE,
            curblock: 0,
            pos: 0,
            blocks: [const { None }; MAX_BLOCKS],
            block_sizes: [const { None }; MAX_BLOCKS],
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

    fn new_block(&self, ctx: &mut SegmentedAllocCtx) {
        ctx.size = ctx.size * GROWTH;
        ctx.curblock += 1;
        let cur_block = ctx.curblock;
        ctx.block_sizes[cur_block] = Some(ctx.size);
        let block = mmap(
            None,
            ctx.size,
            mmap::MmapProt::READ | mmap::MmapProt::WRITE,
            mmap::MmapFlags::PRIVATE | mmap::MmapFlags::ANONYMOUS,
            -1,
            0,
        );
        ctx.blocks[cur_block] = Some(block);
        // eprintln!("created new block at size {}", ctx.size);
    }

    fn request(&self, layout: std::alloc::Layout) -> NonNull<u8> {
        // eprintln!("requesting {:?}", &layout);
        let mut ctx = unsafe { &mut *self.ctx.get() };

        // this is equal to a separate SegmentedAlloc::new impl, but we can't do this in the
        // GlobalAlloc trait since theres only alloc and dealloc.
        if ctx.blocks[0].is_none() {
            ctx.block_sizes[0] = Some(MIN_SIZE);
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

        let offset = ctx.pos.next_multiple_of(layout.align());
        let padded_size = layout.pad_to_align().size();

        if offset + padded_size > ctx.size {
            self.new_block(&mut ctx);
        }

        let Some(block) = ctx.blocks[ctx.curblock] else {
            eprintln!(
                "Attempting to index not allocated block with {:?}, {}",
                layout, self
            );
            std::process::abort();
        };
        let ptr = unsafe { block.add(offset) };
        ctx.pos = offset + padded_size;
        ptr
    }
}

// impl Drop for SegmentedAlloc {
//     fn drop(&mut self) {
//         let ctx = unsafe { &mut *self.ctx.get() };
//         for i in 0..MAX_BLOCKS {
//             let (Some(size), Some(block)) = (ctx.block_sizes[i], ctx.blocks[i]) else {
//                 break;
//             };
//             munmap(block, size);
//         }
//     }
// }

unsafe impl GlobalAlloc for SegmentedAlloc {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        self.request(layout).as_ptr()
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: std::alloc::Layout) {}
}
