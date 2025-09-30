#![allow(dead_code)]

pub const MMAP_SYSCALL: i32 = 9;
pub const MUNMAP_SYSCALL: i32 = 11;

// Not an enum, since READ and WRITE arent mutually exclusive
pub struct MmapProt(i32);

impl MmapProt {
    pub const READ: MmapProt = MmapProt(0x1);
    pub const WRITE: MmapProt = MmapProt(0x2);
    pub fn bits(self) -> i32 {
        self.0
    }
}

impl std::ops::BitOr for MmapProt {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        MmapProt(self.0 | rhs.0)
    }
}

pub struct MmapFlags(i32);

impl MmapFlags {
    pub const PRIVATE: MmapFlags = MmapFlags(0x02);
    pub const ANONYMOUS: MmapFlags = MmapFlags(0x20);
    pub fn bits(self) -> i32 {
        self.0
    }
}

impl std::ops::BitOr for MmapFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        MmapFlags(self.0 | rhs.0)
    }
}

#[inline(always)]
pub fn mmap(
    ptr: Option<*mut u8>,
    length: usize,
    prot: MmapProt,
    flags: MmapFlags,
    fd: i32,
    offset: i64,
) -> *mut u8 {
    let ret: isize;

    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") MMAP_SYSCALL,
            in("rdi") ptr.unwrap_or(std::ptr::null_mut()),
            in("rsi") length,
            in("rdx") prot.bits(),
            in("r10") flags.bits(),
            in("r8")  fd,
            in("r9")  offset,
            lateout("rax") ret,
            options(nostack)
        );
    }
    if ret < 0 {
        panic!("mmap syscall failed: errno={}", -ret);
    }
    ret as *mut u8
}

#[inline(always)]
pub fn munmap(ptr: *mut u8, size: usize) {
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") MMAP_SYSCALL,
            in("rdi") ptr,
            in("rsi") size,
            options(nostack)
        );
    }
}
