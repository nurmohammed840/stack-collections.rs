#![allow(warnings)]
#![feature(allocator_api)]
use core::alloc::{AllocError, Allocator, Layout};
use core::mem::MaybeUninit;
use core::ptr::NonNull;
use std::cell::UnsafeCell;

struct StackAllocator<const N: usize> {
    len: UnsafeCell<usize>,
    buf: UnsafeCell<[MaybeUninit<u8>; N]>,
}

impl<const N: usize> StackAllocator<N> {
    const fn new() -> Self {
        Self {
            len: UnsafeCell::new(0),
            buf: unsafe { MaybeUninit::uninit().assume_init() },
        }
    }
}

unsafe impl<const N: usize> Allocator for StackAllocator<N> {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        unsafe {
            let ptr = self.buf.get() as *mut [_] as *mut [u8];
            println!("allocate: {ptr:?}, {layout:?}");
            Ok(NonNull::new_unchecked(ptr))
        }
    }
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        println!("deallocate: {ptr:?}, {layout:?}");
    }
}

fn main() {
    let salloc: StackAllocator<4096> = StackAllocator::new();
}
