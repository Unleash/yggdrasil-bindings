#![allow(non_camel_case_types)]

/// Adds support for `gettid` and `statx` syscalls.
///
/// This module provides a minimal implementation of the `gettid` and `statx` syscalls for Linux.
/// It uses the `syscall` function to make the system calls directly.
///
/// Note: This does NOT introduce GLIBC_2.28/2.30 by itself. On the contrary, it is designed so that zigbuild is able to build our binaries while only requiring GLIBC_2.25.
///
use core::ffi::{c_long, c_void, c_int};
use std::ffi::c_char;

// Minimal C-ish types without the libc crate
type pid_t = i32;

// Pull in syscall + errno location from the C runtime (glibc),
// but note: this does NOT introduce GLIBC_2.28/2.30 by itself.
extern "C" {
    fn syscall(num: c_long, ...) -> c_long;
    fn __errno_location() -> *mut c_int;
}

// x86_64 syscall numbers
#[cfg(target_arch = "x86_64")]
const SYS_GETTID: c_long = 186;
#[cfg(target_arch = "x86_64")]
const SYS_STATX: c_long = 332;

// aarch64 syscall numbers
#[cfg(target_arch = "aarch64")]
const SYS_GETTID: c_long = 178;
#[cfg(target_arch = "aarch64")]
const SYS_STATX: c_long = 291;


#[no_mangle]
pub unsafe extern "C" fn gettid() -> pid_t {
    syscall(SYS_GETTID) as pid_t
}

#[repr(C)]
pub struct statx_timestamp {
    pub tv_sec: i64,
    pub tv_nsec: u32,
    pub __reserved: i32,
}

#[repr(C)]
pub struct statx {
    pub stx_mask: u32,
    pub stx_blksize: u32,
    pub stx_attributes: u64,
    pub stx_nlink: u32,
    pub stx_uid: u32,
    pub stx_gid: u32,
    pub stx_mode: u16,
    pub __spare0: u16,
    pub stx_ino: u64,
    pub stx_size: u64,
    pub stx_blocks: u64,
    pub stx_attributes_mask: u64,
    pub stx_atime: statx_timestamp,
    pub stx_btime: statx_timestamp,
    pub stx_ctime: statx_timestamp,
    pub stx_mtime: statx_timestamp,
    pub stx_rdev_major: u32,
    pub stx_rdev_minor: u32,
    pub stx_dev_major: u32,
    pub stx_dev_minor: u32,
    pub stx_mnt_id: u64,
    pub stx_dio_mem_align: u32,
    pub stx_dio_offset_align: u32,
    pub __spare3: [u64; 12],
}

#[no_mangle]
pub unsafe extern "C" fn statx(
    dirfd: c_int,
    pathname: *const c_char,
    flags: c_int,
    mask: u32,
    buf: *mut statx,
) -> c_int {
    let ret = syscall(SYS_STATX, dirfd, pathname, flags, mask, buf as *mut c_void);

    if ret < 0 {
        // syscall returns -errno
        *__errno_location() = (-ret) as c_int;
        -1
    } else {
        ret as c_int
    }
}
