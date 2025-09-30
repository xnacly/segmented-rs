use std::{alloc::GlobalAlloc, cell::RefCell};

const MIN_SIZE: usize = 4096;
const MAX_BLOCKS: usize = 55;
const GROWTH: usize = 2;

struct SegmentedAllocCtx {
    /// idx into self.blocks
    curblock: usize,
    /// size of the current block
    size: usize,
    /// bytes in use of the current block
    pos: usize,
    blocks: [Option<*mut u8>; MAX_BLOCKS],
    block_sizes: [Option<usize>; MAX_BLOCKS],
}

/// Implements a variable size bump allocator, employing mmap to allocate a starting block of
/// 4096B, once a block is exceeded by a request, the allocator mmaps a new block double the size
/// of the previously allocated block
pub struct SegmentedAlloc {
    ctx: RefCell<SegmentedAllocCtx>,
}

impl SegmentedAlloc {
    fn new_block(&self) {
        let mut ctx = self.ctx.borrow_mut();
        let new_size = ctx.size * GROWTH;
        let pos = ctx.curblock;
        ctx.block_sizes[pos] = Some(new_size);
        ctx.blocks[pos] = Some(&mut u8::default());
        ctx.curblock += 1;
        todo!("mmap")
    }

    fn request(&self, layout: std::alloc::Layout) -> *mut u8 {
        let mut ctx = self.ctx.borrow_mut();
        // this is equal to a separate SegmentedAlloc::new impl, but we can't do this in the
        // GlobalAlloc trait since theres only alloc and dealloc.
        if ctx.blocks[0].is_none() {
            ctx.size = MIN_SIZE;
            self.new_block();
        }

        let offset = ctx.pos.next_multiple_of(layout.align());
        let padded_size = layout.pad_to_align().size();

        if offset + padded_size > ctx.size {
            self.new_block();
            return self.request(layout);
        }

        let block = ctx.blocks[ctx.curblock].expect("block is not allocated");
        let ptr = unsafe { block.add(offset) };
        ctx.pos = offset + padded_size;
        ptr
    }
}

impl Drop for SegmentedAlloc {
    fn drop(&mut self) {
        let ctx = self.ctx.borrow_mut();
        for i in 0..MAX_BLOCKS {
            let Some(size) = ctx.block_sizes[i] else {
                break;
            };

            let Some(block) = ctx.blocks[i] else {
                break;
            };

            todo!("unmap")
        }
    }
}

unsafe impl GlobalAlloc for SegmentedAlloc {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        self.request(layout)
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: std::alloc::Layout) {
        ()
    }
}
