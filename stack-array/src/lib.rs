#![no_std]

//! This library provides an array type that is similar to the built-in arr type, but lives on the stack!
//!
//! You can store a fixed number of elements of a specific type (even non-copy types!)

mod drain;

use core::{
    fmt, mem,
    mem::MaybeUninit,
    ops::{Bound, Deref, DerefMut, RangeBounds},
    ptr,
    ptr::NonNull,
    slice,
};

use drain::Drain;

/// A data structure for storing and manipulating fixed number of elements of a specific type.
pub struct Array<T, const N: usize> {
    len: usize,
    buf: [MaybeUninit<T>; N],
}

impl<T, const N: usize> Array<T, N> {
    /// Creates a new [`Array<T, N>`].
    ///
    /// # Examples
    ///
    /// ```
    /// use stack_array::Array;
    ///
    /// let arr = Array::<u8, 4>::new();
    /// // or
    /// let arr: Array<u8, 4> = Array::new();
    /// ```
    #[inline]
    pub fn new() -> Self {
        Self {
            len: 0,
            // SAFETY: An uninitialized `[MaybeUninit<_>; N]` is valid.
            buf: unsafe { MaybeUninit::uninit().assume_init() },
        }
    }

    /// Returns the number of elements the array can hold.
    ///
    /// # Examples
    ///
    /// ```
    /// use stack_array::Array;
    ///
    /// let arr: Array<u8, 4> = Array::new();
    /// assert_eq!(arr.capacity(), 4);
    /// ```
    #[inline]
    pub const fn capacity(&self) -> usize {
        N
    }

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

    /// Returns the number of elements can be inserted into the array.
    ///
    /// # Examples
    ///
    /// ```
    /// use stack_array::Array;
    ///
    /// let arr: Array<u8, 3> = Array::from([1, 2]);
    /// assert_eq!(arr.remaining_capacity(), 1);
    /// ```
    #[inline]
    pub const fn remaining_capacity(&self) -> usize {
        N - self.len
    }

    /// Shortens the array, keeping the first `len` elements and dropping
    /// the rest.
    ///
    /// If `len` is greater than the array's current length, this has no
    /// effect.
    ///
    /// The [`drain`] method can emulate `truncate`, but causes the excess
    /// elements to be returned instead of dropped.
    ///
    /// # Examples
    ///
    /// Truncating a five element array to two elements:
    ///
    /// ```
    /// use stack_array::Array;
    ///
    /// let mut arr: Array<u8, 5> = Array::from([1, 2, 3, 4, 5]);
    /// arr.truncate(2);
    /// assert_eq!(arr[..], [1, 2]);
    /// ```
    ///
    /// No truncation occurs when `len` is greater than the array's current
    /// length:
    ///
    /// ```
    /// use stack_array::Array;
    ///
    /// let mut arr: Array<u8, 5> = Array::from([1, 2, 3]);
    /// arr.truncate(8);
    /// assert_eq!(arr[..], [1, 2, 3]);
    /// ```
    ///
    /// Truncating when `len == 0` is equivalent to calling the [`clear`]
    /// method.
    ///
    /// ```
    /// use stack_array::Array;
    ///
    /// let mut arr: Array<u8, 5> = Array::from([1, 2, 3]);
    /// arr.truncate(0);
    /// assert_eq!(arr[..], []);
    /// ```
    ///
    /// [`clear`]: Array::clear
    /// [`drain`]: Array::drain
    pub fn truncate(&mut self, len: usize) {
        // This is safe because:
        //
        // * the slice passed to `drop_in_place` is valid; the `len > self.len`
        //   case avoids creating an invalid slice, and
        // * the `len` of the array is shrunk before calling `drop_in_place`,
        //   such that no value will be dropped twice in case `drop_in_place`
        //   were to panic once (if it panics twice, the program aborts).
        unsafe {
            // Note: It's intentional that this is `>` and not `>=`.
            //       Changing it to `>=` has negative performance
            //       implications in some cases. See #78884 for more.
            if len > self.len {
                return;
            }
            let remaining_len = self.len - len;
            let s = ptr::slice_from_raw_parts_mut(self.as_mut_ptr().add(len), remaining_len);
            self.len = len;
            ptr::drop_in_place(s);
        }
    }

    /// Extracts a slice containing the entire array.
    ///
    /// Equivalent to `&s[..]`.
    #[inline]
    pub fn as_slice(&self) -> &[T] {
        self
    }

    /// Extracts a mutable slice of the entire array.
    ///
    /// Equivalent to `&mut s[..]`.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        self
    }

    /// Returns a raw pointer to the array's buffer.
    ///
    /// The caller must ensure that the array outlives the pointer this
    /// function returns, or else it will end up pointing to garbage.
    /// Modifying the array may cause its buffer to be reallocated,
    /// which would also make any pointers to it invalid.
    ///
    /// The caller must also ensure that the memory the pointer (non-transitively) points to
    /// is never written to (except inside an `UnsafeCell`) using this pointer or any pointer
    /// derived from it. If you need to mutate the contents of the slice, use [`as_mut_ptr`].
    ///
    /// [`as_mut_ptr`]: Array::as_mut_ptr
    #[inline]
    pub fn as_ptr(&self) -> *const T {
        // We shadow the slice method of the same name to avoid going through
        // `deref`, which creates an intermediate reference.
        self.buf.as_ptr() as _
    }

    /// Returns an unsafe mutable pointer to the array's buffer.
    ///
    /// The caller must ensure that the array outlives the pointer this
    /// function returns, or else it will end up pointing to garbage.
    /// Modifying the array may cause its buffer to be reallocated,
    /// which would also make any pointers to it invalid.
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut T {
        // We shadow the slice method of the same name to avoid going through
        // `deref_mut`, which creates an intermediate reference.
        self.buf.as_mut_ptr() as _
    }

    /// Returns an unsafe mutable pointer to the array's buffer.
    ///
    /// The caller must ensure that the array outlives the pointer this
    /// function returns, or else it will end up pointing to garbage.
    /// Modifying the array may cause its buffer to be reallocated,
    /// which would also make any pointers to it invalid.
    #[inline]
    pub unsafe fn set_len(&mut self, new_len: usize) {
        debug_assert!(new_len <= self.capacity());
        self.len = new_len;
    }

    /// Removes an element from the array and returns it.
    ///
    /// The removed element is replaced by the last element of the array.
    ///
    /// This does not preserve ordering, but is *O*(1).
    /// If you need to preserve the element order, use [`remove`] instead.
    ///
    /// [`remove`]: Array::remove
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use stack_array::Array;
    ///
    /// let mut arr: Array<&str, 4> = Array::from(["foo", "bar", "baz", "qux"]);
    ///
    /// assert_eq!(arr.swap_remove(1), "bar");
    /// assert_eq!(arr[..], ["foo", "qux", "baz"]);
    ///
    /// assert_eq!(arr.swap_remove(0), "foo");
    /// assert_eq!(arr[..], ["baz", "qux"]);
    /// ```
    #[inline]
    pub fn swap_remove(&mut self, index: usize) -> T {
        #[cold]
        #[inline(never)]
        fn assert_failed(index: usize, len: usize) -> ! {
            panic!(
                "swap_remove index (is {}) should be < len (is {})",
                index, len
            );
        }
        let len = self.len();
        if index >= len {
            assert_failed(index, len);
        }
        unsafe {
            // We replace self[index] with the last element. Note that if the
            // bounds check above succeeds there must be a last element (which
            // can be self[index] itself).
            let value = ptr::read(self.as_ptr().add(index));
            let base_ptr = self.as_mut_ptr();
            ptr::copy(base_ptr.add(len - 1), base_ptr.add(index), 1);
            self.set_len(len - 1);
            value
        }
    }

    /// Inserts an element at position index within the array, shifting all elements after it to the right.
    /// # Examples
    ///
    /// ```
    /// use stack_array::Array;
    ///
    /// let mut list: Array<u8, 3> = Array::from([3]);
    /// list.insert(0, 1);
    /// assert_eq!(&list[..], [1, 3]);
    /// list.insert(1, 2);
    /// assert_eq!(&list[..], [1, 2, 3]);
    /// ```
    ///
    /// # Panics
    /// Panics if the index is out of bounds.
    pub fn insert(&mut self, index: usize, element: T) {
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

    /// Removes an element from position index within the array, shifting all elements after it to the left.
    ///
    /// Note: Because this shifts over the remaining elements, it has a
    /// worst-case performance of *O*(*n*). If you don't need the order of elements
    /// to be preserved, use [`swap_remove`] instead.
    ///
    /// [`swap_remove`]: Array::swap_remove
    ///
    /// # Examples
    ///
    /// ```
    /// use stack_array::Array;
    ///
    /// let mut list: Array<u8, 3> = Array::from([1, 2, 3]);
    /// assert_eq!(list.remove(0), 1);
    /// assert_eq!(list.remove(0), 2);
    /// assert_eq!(list.remove(0), 3);
    /// ```
    ///
    /// # Panics
    /// Panics if the index is out of bounds.
    pub fn remove(&mut self, index: usize) -> T {
        #[cold]
        #[inline(never)]
        #[track_caller]
        fn assert_failed(index: usize, len: usize) -> ! {
            panic!("removal index (is {}) should be < len (is {})", index, len);
        }

        let len = self.len();
        if index >= len {
            assert_failed(index, len);
        }
        unsafe {
            // infallible
            let ret;
            {
                // the place we are taking from.
                let ptr = self.as_mut_ptr().add(index);
                // copy it out, unsafely having a copy of the value on
                // the stack and in the array at the same time.
                ret = ptr::read(ptr);
                // Shift everything down to fill in that spot.
                ptr::copy(ptr.offset(1), ptr, len - index - 1);
            }
            self.set_len(len - 1);
            ret
        }
    }

    /// Retains only the elements specified by the predicate.
    ///
    /// In other words, remove all elements `e` such that `f(&e)` returns `false`.
    /// This method operates in place, visiting each element exactly once in the
    /// original order, and preserves the order of the retained elements.
    ///
    /// # Examples
    ///
    /// ```
    /// use stack_array::Array;
    ///
    /// let mut arr: Array<u8, 4> = Array::from([1, 2, 3, 4]);
    ///
    /// arr.retain(|x| *x % 2 == 0);
    /// assert_eq!(arr[..], [2, 4]);
    /// ```
    ///
    /// Because the elements are visited exactly once in the original order,
    /// external state may be used to decide which elements to keep.
    ///
    /// ```
    /// use stack_array::Array;
    ///
    /// let mut arr: Array<u8, 5> = Array::from([1, 2, 3, 4, 5]);
    /// let keep = [false, true, true, false, true];
    /// let mut iter = keep.iter();
    /// arr.retain(|_| *iter.next().unwrap());
    /// assert_eq!(arr[..], [2, 3, 5]);
    /// ```
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut T) -> bool,
    {
        let original_len = self.len();
        // Avoid double drop if the drop guard is not executed,
        // since we may make some holes during the process.
        unsafe { self.set_len(0) };

        // arr: [Kept, Kept, Hole, Hole, Hole, Hole, Unchecked, Unchecked]
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

        // process_one return a bool indicates whether the processing element should be retained.
        #[inline]
        fn process_one<F, T, const N: usize, const DELETED: bool>(
            f: &mut F,
            g: &mut BackshiftOnDrop<'_, T, N>,
        ) -> bool
        where
            F: FnMut(&mut T) -> bool,
        {
            // SAFETY: Unchecked element must be valid.
            let cur = unsafe { &mut *g.v.as_mut_ptr().add(g.processed_len) };
            if !f(cur) {
                // Advance early to avoid double drop if `drop_in_place` panicked.
                g.processed_len += 1;
                g.deleted_cnt += 1;
                // SAFETY: We never touch this element again after dropped.
                unsafe { ptr::drop_in_place(cur) };
                // We already advanced the counter.
                return false;
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
            return true;
        }

        // Stage 1: Nothing was deleted.
        while g.processed_len != original_len {
            if !process_one::<F, T, N, false>(&mut f, &mut g) {
                break;
            }
        }

        // Stage 2: Some elements were deleted.
        while g.processed_len != original_len {
            process_one::<F, T, N, true>(&mut f, &mut g);
        }

        // All item are processed. This can be optimized to `set_len` by LLVM.
        drop(g);
    }

    /// Removes all but the first of consecutive elements in the array that resolve to the same
    /// key.
    ///
    /// If the array is sorted, this removes all duplicates.
    ///
    /// # Examples
    ///
    /// ```
    /// use stack_array::Array;
    ///
    /// let mut arr: Array<u8, 5> = Array::from([10, 20, 21, 30, 20]);
    ///
    /// arr.dedup_by_key(|i| *i / 10);
    ///
    /// assert_eq!(arr[..], [10, 20, 30, 20]);
    #[inline]
    pub fn dedup_by_key<F, K>(&mut self, mut key: F)
    where
        F: FnMut(&mut T) -> K,
        K: PartialEq,
    {
        self.dedup_by(|a, b| key(a) == key(b))
    }

    /// Removes all but the first of consecutive elements in the array satisfying a given equality
    /// relation.
    ///
    /// The `same_bucket` function is passed references to two elements from the array and
    /// must determine if the elements compare equal. The elements are passed in opposite order
    /// from their order in the slice, so if `same_bucket(a, b)` returns `true`, `a` is removed.
    ///
    /// If the array is sorted, this removes all duplicates.
    ///
    /// # Examples
    ///
    /// ```
    /// use stack_array::Array;
    ///
    /// let mut arr: Array<&str, 5> = Array::from(["foo", "bar", "Bar", "baz", "bar"]);
    ///
    /// arr.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    ///
    /// assert_eq!(arr[..], ["foo", "bar", "baz", "bar"]);
    /// ```
    pub fn dedup_by<F>(&mut self, mut same_bucket: F)
    where
        F: FnMut(&mut T, &mut T) -> bool,
    {
        let len = self.len();
        if len <= 1 {
            return;
        }

        /* INVARIANT: arr.len() > read >= write > write-1 >= 0 */
        struct FillGapOnDrop<'a, T, const N: usize> {
            /* Offset of the element we want to check if it is duplicate */
            read: usize,

            /* Offset of the place where we want to place the non-duplicate
             * when we find it. */
            write: usize,

            /* The arr that would need correction if `same_bucket` panicked */
            arr: &'a mut Array<T, N>,
        }

        impl<'a, T, const N: usize> Drop for FillGapOnDrop<'a, T, N> {
            fn drop(&mut self) {
                /* This code gets executed when `same_bucket` panics */

                /* SAFETY: invariant guarantees that `read - write`
                 * and `len - read` never overflow and that the copy is always
                 * in-bounds. */
                unsafe {
                    let ptr = self.arr.as_mut_ptr();
                    let len = self.arr.len();

                    /* How many items were left when `same_bucket` paniced.
                     * Basically arr[read..].len() */
                    let items_left = len.wrapping_sub(self.read);

                    /* Pointer to first item in arr[write..write+items_left] slice */
                    let dropped_ptr = ptr.add(self.write);
                    /* Pointer to first item in arr[read..] slice */
                    let valid_ptr = ptr.add(self.read);

                    /* Copy `arr[read..]` to `arr[write..write+items_left]`.
                     * The slices can overlap, so `copy_nonoverlapping` cannot be used */
                    ptr::copy(valid_ptr, dropped_ptr, items_left);

                    /* How many items have been already dropped
                     * Basically arr[read..write].len() */
                    let dropped = self.read.wrapping_sub(self.write);

                    self.arr.set_len(len - dropped);
                }
            }
        }

        let mut gap = FillGapOnDrop {
            read: 1,
            write: 1,
            arr: self,
        };
        let ptr = gap.arr.as_mut_ptr();

        /* Drop items while going through arr, it should be more efficient than
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
            gap.arr.set_len(gap.write);
            mem::forget(gap);
        }
    }

    /// Appends an element to the back of a collection
    ///
    /// ### Examples
    ///
    /// ```rust
    /// use stack_array::Array;
    ///
    /// let mut arr: Array<u8, 3> = Array::from([1]);
    /// arr.push(2);
    /// arr.push(3);
    /// assert_eq!(&arr[..], [1, 2, 3]);
    /// ```
    ///
    /// # Panics
    /// Panics if the array is full.
    #[inline]
    pub fn push(&mut self, value: T) {
        if self.is_full() {
            panic!("Array is full")
        }
        unsafe {
            let end = self.as_mut_ptr().add(self.len);
            ptr::write(end, value);
            self.len += 1;
        }
    }

    /// Removes the last element from a collection and returns it.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use stack_array::Array;
    ///
    /// let mut arr: Array<u8, 3> = Array::from([1, 2]);
    /// assert_eq!(arr.pop(), 2);
    /// assert_eq!(arr.pop(), 1);
    /// assert!(arr.is_empty());
    /// ```
    ///
    /// # Panics
    /// Panics if the array is empty.
    #[inline]
    pub fn pop(&mut self) -> T {
        unsafe {
            self.len -= 1;
            ptr::read(self.as_ptr().add(self.len()))
        }
    }

    /// Moves all the elements of `other` into `Self`
    ///
    /// # Panics
    ///
    /// Panics if the number of elements in the array overflows.
    ///
    /// # Examples
    ///
    /// ```
    /// use stack_array::Array;
    ///
    /// let mut arr: Array<u8, 6> = Array::from([1, 2, 3]);
    /// arr.append([4, 5, 6]);
    /// assert_eq!(arr[..], [1, 2, 3, 4, 5, 6]);
    /// ```
    #[inline]
    pub fn append(&mut self, other: impl AsRef<[T]>)
    where
        T: Copy,
    {
        let other = other.as_ref();
        let count = other.len();
        if self.remaining_capacity() < count {
            panic!("Array is full")
        }
        let len = self.len();
        unsafe { ptr::copy_nonoverlapping(other.as_ptr(), self.as_mut_ptr().add(len), count) };
        self.len += count;
    }

    /// Creates a draining iterator that removes the specified range in the array
    /// and yields the removed items.
    ///
    /// When the iterator **is** dropped, all elements in the range are removed
    /// from the array, even if the iterator was not fully consumed. If the
    /// iterator **is not** dropped (with [`mem::forget`] for example), it is
    /// unspecified how many elements are removed.
    ///
    /// # Panics
    ///
    /// Panics if the starting point is greater than the end point or if
    /// the end point is greater than the length of the array.
    ///
    /// # Examples
    ///
    /// ```
    /// use stack_array::Array;
    ///
    /// let mut arr: Array<u8, 3> = Array::from([1, 2, 3]);
    /// let vec: Vec<_> = arr.drain(1..).collect();
    /// assert_eq!(arr[..], [1]);
    /// assert_eq!(vec[..], [2, 3]);
    ///
    /// // A full range clears the array
    /// arr.drain(..);
    /// assert_eq!(arr[..], []);
    /// ```
    pub fn drain<R>(&mut self, range: R) -> Drain<'_, T, N>
    where
        R: RangeBounds<usize>,
    {
        // Memory safety
        //
        // When the Drain is first created, it shortens the length of
        // the source array to make sure no uninitialized or moved-from elements
        // are accessible at all if the Drain's destructor never gets to run.
        //
        // Drain will ptr::read out the values to remove.
        // When finished, remaining tail of the arr is copied back to cover
        // the hole, and the array length is restored to the new length.
        //
        let len = self.len();
        let start = match range.start_bound() {
            Bound::Unbounded => 0,
            Bound::Included(&i) => i,
            Bound::Excluded(&i) => i.saturating_add(1),
        };
        let end = match range.end_bound() {
            Bound::Excluded(&j) => j,
            Bound::Included(&j) => j.saturating_add(1),
            Bound::Unbounded => len,
        };

        unsafe {
            // set self.arr length's to start, to be safe in case Drain is leaked
            self.set_len(start);
            // Use the borrow in the IterMut to indicate borrowing behavior of the
            // whole Drain iterator (like &mut T).
            let range_slice = slice::from_raw_parts_mut(self.as_mut_ptr().add(start), end - start);
            Drain {
                tail_start: end,
                tail_len: len - end,
                iter: range_slice.iter(),
                arr: NonNull::from(self),
            }
        }
    }

    /// Clears the array, removing all values.
    ///
    /// # Examples
    ///
    /// ```
    /// use stack_array::Array;
    ///
    /// let mut list: Array<u8, 3> = Array::from([1, 2, 3]);
    /// list.clear();
    /// assert!(list.is_empty());
    /// ```
    #[inline]
    pub fn clear(&mut self) {
        self.truncate(0)
    }

    /// Returns the number of elements currently in the array.
    ///
    /// # Examples
    ///
    /// ```
    /// use stack_array::Array;
    ///     
    /// let arr: Array<u8, 3> = Array::from([1, 2]);
    /// assert_eq!(arr.len(), 2);
    /// ```
    #[inline]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns true if the array contains no elements.
    ///
    /// # Examples
    ///
    /// ```
    /// use stack_array::Array;
    ///
    /// let mut arr: Array<u8, 2> = Array::new();
    /// assert!(arr.is_empty());
    ///
    /// arr.push(1);
    /// assert!(!arr.is_empty());
    /// ```
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<T, const N: usize> Default for Array<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> AsRef<[T]> for Array<T, N> {
    #[inline]
    fn as_ref(&self) -> &[T] {
        // SAFETY: slice will contain only initialized objects.
        unsafe { &*(&self.buf[..self.len] as *const [MaybeUninit<T>] as *const [T]) }
    }
}

impl<T, const N: usize> AsMut<[T]> for Array<T, N> {
    #[inline]
    fn as_mut(&mut self) -> &mut [T] {
        // SAFETY: slice will contain only initialized objects.
        unsafe { &mut *(&mut self.buf[..self.len] as *mut [MaybeUninit<T>] as *mut [T]) }
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
        f.debug_struct("Array")
            .field("len", &self.len)
            .field("buf", &self.as_ref())
            .finish()
    }
}

impl<T: Copy, const N: usize> From<&[T]> for Array<T, N> {
    fn from(values: &[T]) -> Self {
        let mut array = Self::new();
        array.append(values);
        array
    }
}

impl<T, const N: usize, const S: usize> From<[T; S]> for Array<T, N> {
    fn from(values: [T; S]) -> Self {
        let mut array = Self::new();
        for v in values {
            array.push(v);
        }
        array
    }
}
