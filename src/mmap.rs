#![allow(dead_code)]

const MMAP_SYSCALL: i32 = 9;
const MUNMAP_SYSCALL: i32 = 11;

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

pub fn mmap(
    ptr: Option<std::ptr::NonNull<u8>>,
    length: usize,
    prot: MmapProt,
    flags: MmapFlags,
    fd: i32,
    offset: i64,
) -> std::ptr::NonNull<u8> {
    let ret: isize;

    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") MMAP_SYSCALL,
            in("rdi") ptr.map(|nn| nn.as_ptr()).unwrap_or(std::ptr::null_mut()),
            in("rsi") length,
            in("rdx") prot.bits(),
            in("r10") flags.bits(),
            in("r8")  fd,
            in("r9")  offset,
            lateout("rax") ret,
            options(nostack, preserves_flags, readonly)
        );
    }
    if ret < 0 {
        let errno = -ret;
        eprintln!(
            "mmap failed (errno {}): {}",
            errno,
            std::io::Error::from_raw_os_error(errno as i32)
        );
        std::process::abort()
    }

    unsafe { std::ptr::NonNull::new_unchecked(ret as *mut u8) }
}

pub fn munmap(ptr: std::ptr::NonNull<u8>, size: usize) {
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") MUNMAP_SYSCALL,
            in("rdi") ptr.as_ptr(),
            in("rsi") size,
            lateout("rax") ret,
            options(nostack, preserves_flags, readonly)
        );
    }

    if ret < 0 {
        let errno = -ret;
        eprintln!(
            "munmap failed (errno {}): {}",
            errno,
            std::io::Error::from_raw_os_error(errno as i32)
        );
        std::process::abort()
    }
}
