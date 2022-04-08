//! This library provides an array type that is similar to the built-in arr type, but lives on the stack!

#![allow(warnings)]
// #![no_std]


// mod drain;
mod interface;

// use drain::Drain;
pub use interface::ArrayInterface;

use core::{
    fmt, mem,
    mem::MaybeUninit,
    ops::{Bound, Deref, DerefMut, RangeBounds},
    ptr,
    ptr::NonNull,
    slice,
};

/// A data structure for storing and manipulating fixed number of elements of a specific type.
pub struct Array<T, const N: usize> {
    len: usize,
    buf: [MaybeUninit<T>; N],
}

impl<T, const N: usize> ArrayInterface<T> for Array<T, N> {
    #[inline]
    fn new() -> Self {
        Self {
            len: 0,
            buf: unsafe { MaybeUninit::uninit().assume_init() },
        }
    }

    #[inline]
    fn capacity(&self) -> usize {
        N
    }

    #[inline]
    fn as_ptr(&self) -> *const T {
        self.buf.as_ptr() as _
    }

    #[inline]
    fn as_mut_ptr(&mut self) -> *mut T {
        self.buf.as_mut_ptr() as _
    }

    #[inline]
    fn set_len(&mut self, new_len: usize) {
        debug_assert!(new_len <= self.capacity());
        self.len = new_len;
    }

    fn insert(&mut self, index: usize, element: T) {
        #[cold]
        #[inline(never)]
        fn assert_failed(index: usize, len: usize) -> ! {
            panic!(
                "insertion index (is {}) should be <= len (is {})",
                index, len
            );
        }

        let len = self.len();
        if index > len {
            assert_failed(index, len);
        }

        // space for the new element
        if self.is_full() {
            panic!("array is full");
        }

        unsafe {
            // infallible
            // The spot to put the new value
            {
                let p = self.as_mut_ptr().add(index);
                // Shift everything over to make space. (Duplicating the
                // `index`th element into two consecutive places.)
                ptr::copy(p, p.offset(1), len - index);
                // Write it in, overwriting the first copy of the `index`th
                // element.
                ptr::write(p, element);
            }
            self.set_len(len + 1);
        }
    }

    fn retain_mut<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut T) -> bool,
    {
        let original_len = self.len();
        // Avoid double drop if the drop guard is not executed,
        // since we may make some holes during the process.
        unsafe { self.set_len(0) };

        // Vec: [Kept, Kept, Hole, Hole, Hole, Hole, Unchecked, Unchecked]
        //      |<-              processed len   ->| ^- next to check
        //                  |<-  deleted cnt     ->|
        //      |<-              original_len                          ->|
        // Kept: Elements which predicate returns true on.
        // Hole: Moved or dropped element slot.
        // Unchecked: Unchecked valid elements.
        //
        // This drop guard will be invoked when predicate or `drop` of element panicked.
        // It shifts unchecked elements to cover holes and `set_len` to the correct length.
        // In cases when predicate and `drop` never panick, it will be optimized out.
        struct BackshiftOnDrop<'a, T, const N: usize> {
            v: &'a mut Array<T, N>,
            processed_len: usize,
            deleted_cnt: usize,
            original_len: usize,
        }

        impl<T, const N: usize> Drop for BackshiftOnDrop<'_, T, N> {
            fn drop(&mut self) {
                if self.deleted_cnt > 0 {
                    // SAFETY: Trailing unchecked items must be valid since we never touch them.
                    unsafe {
                        ptr::copy(
                            self.v.as_ptr().add(self.processed_len),
                            self.v
                                .as_mut_ptr()
                                .add(self.processed_len - self.deleted_cnt),
                            self.original_len - self.processed_len,
                        );
                    }
                }
                // SAFETY: After filling holes, all items are in contiguous memory.
                unsafe {
                    self.v.set_len(self.original_len - self.deleted_cnt);
                }
            }
        }

        let mut g = BackshiftOnDrop {
            v: self,
            processed_len: 0,
            deleted_cnt: 0,
            original_len,
        };

        fn process_loop<F, T, const N: usize, const DELETED: bool>(
            original_len: usize,
            f: &mut F,
            g: &mut BackshiftOnDrop<'_, T, N>,
        ) where
            F: FnMut(&mut T) -> bool,
        {
            while g.processed_len != original_len {
                // SAFETY: Unchecked element must be valid.
                let cur = unsafe { &mut *g.v.as_mut_ptr().add(g.processed_len) };
                if !f(cur) {
                    // Advance early to avoid double drop if `drop_in_place` panicked.
                    g.processed_len += 1;
                    g.deleted_cnt += 1;
                    // SAFETY: We never touch this element again after dropped.
                    unsafe { ptr::drop_in_place(cur) };
                    // We already advanced the counter.
                    if DELETED {
                        continue;
                    } else {
                        break;
                    }
                }
                if DELETED {
                    // SAFETY: `deleted_cnt` > 0, so the hole slot must not overlap with current element.
                    // We use copy for move, and never touch this element again.
                    unsafe {
                        let hole_slot = g.v.as_mut_ptr().add(g.processed_len - g.deleted_cnt);
                        ptr::copy_nonoverlapping(cur, hole_slot, 1);
                    }
                }
                g.processed_len += 1;
            }
        }

        // Stage 1: Nothing was deleted.
        process_loop::<F, T, N, false>(original_len, &mut f, &mut g);

        // Stage 2: Some elements were deleted.
        process_loop::<F, T, N, true>(original_len, &mut f, &mut g);

        // All item are processed. This can be optimized to `set_len` by LLVM.
        drop(g);
    }

    fn dedup_by<F>(&mut self, mut same_bucket: F)
    where
        F: FnMut(&mut T, &mut T) -> bool,
    {
        let len = self.len();
        if len <= 1 {
            return;
        }

        /* INVARIANT: vec.len() > read >= write > write-1 >= 0 */
        struct FillGapOnDrop<'a, T, const N: usize> {
            /* Offset of the element we want to check if it is duplicate */
            read: usize,

            /* Offset of the place where we want to place the non-duplicate
             * when we find it. */
            write: usize,

            /* The Vec that would need correction if `same_bucket` panicked */
            vec: &'a mut Array<T, N>,
        }

        impl<'a, T, const N: usize> Drop for FillGapOnDrop<'a, T, N> {
            fn drop(&mut self) {
                /* This code gets executed when `same_bucket` panics */

                /* SAFETY: invariant guarantees that `read - write`
                 * and `len - read` never overflow and that the copy is always
                 * in-bounds. */
                unsafe {
                    let ptr = self.vec.as_mut_ptr();
                    let len = self.vec.len();

                    /* How many items were left when `same_bucket` panicked.
                     * Basically vec[read..].len() */
                    let items_left = len.wrapping_sub(self.read);

                    /* Pointer to first item in vec[write..write+items_left] slice */
                    let dropped_ptr = ptr.add(self.write);
                    /* Pointer to first item in vec[read..] slice */
                    let valid_ptr = ptr.add(self.read);

                    /* Copy `vec[read..]` to `vec[write..write+items_left]`.
                     * The slices can overlap, so `copy_nonoverlapping` cannot be used */
                    ptr::copy(valid_ptr, dropped_ptr, items_left);

                    /* How many items have been already dropped
                     * Basically vec[read..write].len() */
                    let dropped = self.read.wrapping_sub(self.write);

                    self.vec.set_len(len - dropped);
                }
            }
        }

        let mut gap = FillGapOnDrop {
            read: 1,
            write: 1,
            vec: self,
        };
        let ptr = gap.vec.as_mut_ptr();

        /* Drop items while going through Vec, it should be more efficient than
         * doing slice partition_dedup + truncate */

        /* SAFETY: Because of the invariant, read_ptr, prev_ptr and write_ptr
         * are always in-bounds and read_ptr never aliases prev_ptr */
        unsafe {
            while gap.read < len {
                let read_ptr = ptr.add(gap.read);
                let prev_ptr = ptr.add(gap.write.wrapping_sub(1));

                if same_bucket(&mut *read_ptr, &mut *prev_ptr) {
                    // Increase `gap.read` now since the drop may panic.
                    gap.read += 1;
                    /* We have found duplicate, drop it in-place */
                    ptr::drop_in_place(read_ptr);
                } else {
                    let write_ptr = ptr.add(gap.write);

                    /* Because `read_ptr` can be equal to `write_ptr`, we either
                     * have to use `copy` or conditional `copy_nonoverlapping`.
                     * Looks like the first option is faster. */
                    ptr::copy(read_ptr, write_ptr, 1);

                    /* We have filled that place, so go further */
                    gap.write += 1;
                    gap.read += 1;
                }
            }

            /* Technically we could let `gap` clean up with its Drop, but
             * when `same_bucket` is guaranteed to not panic, this bloats a little
             * the codegen, so we just do it manually */
            gap.vec.set_len(gap.write);
            mem::forget(gap);
        }
    }

    #[inline]
    fn push(&mut self, value: T) {
        if self.is_full() {
            panic!("Array is full")
        }
        unsafe {
            let end = self.as_mut_ptr().add(self.len);
            ptr::write(end, value);
            self.len += 1;
        }
    }

    #[inline]
    fn len(&self) -> usize {
        self.len
    }

    #[inline]
    fn pop(&mut self) -> T {
        unsafe {
            self.len -= 1;
            ptr::read(self.as_ptr().add(self.len()))
        }
    }
}

impl<T, const N: usize> Array<T, N> {
    /// Returns `true`, If the array is full.
    ///
    /// # Examples
    ///
    /// ```
    /// use stack_array::Array;
    ///
    /// let arr: Array<u8, 3> = Array::from([1, 2]);
    /// assert!(!arr.is_full());
    /// ```
    #[inline]
    pub const fn is_full(&self) -> bool {
        N == self.len
    }
}

impl<T, const N: usize> Default for Array<T, N> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> AsRef<[T]> for Array<T, N> {
    #[inline]
    fn as_ref(&self) -> &[T] {
        // SAFETY: slice will contain only initialized objects.
        unsafe { &*(self.buf.get_unchecked(..self.len) as *const [MaybeUninit<T>] as *const [T]) }
    }
}

impl<T, const N: usize> AsMut<[T]> for Array<T, N> {
    #[inline]
    fn as_mut(&mut self) -> &mut [T] {
        // SAFETY: slice will contain only initialized objects.
        unsafe {
            &mut *(self.buf.get_unchecked_mut(..self.len) as *mut [MaybeUninit<T>] as *mut [T])
        }
    }
}

impl<T, const N: usize> Deref for Array<T, N> {
    type Target = [T];
    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<T, const N: usize> DerefMut for Array<T, N> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

impl<T, const N: usize> Drop for Array<T, N> {
    fn drop(&mut self) {
        self.clear();
    }
}

impl<T: fmt::Debug, const N: usize> fmt::Debug for Array<T, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

// impl<T: Copy, const N: usize> From<&[T]> for Array<T, N> {
//     fn from(values: &[T]) -> Self {
//         let mut array = Self::new();
//         array.append(values);
//         array
//     }
// }

// impl<T, const N: usize, const S: usize> From<[T; S]> for Array<T, N> {
//     fn from(values: [T; S]) -> Self {
//         let mut array = Self::new();
//         for v in values {
//             array.push(v);
//         }
//         array
//     }
// }
