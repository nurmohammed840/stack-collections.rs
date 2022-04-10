use crate::{drain::slice_range, *};

pub trait Array<T>: AsRef<[T]> + AsMut<[T]> + Default {
    /// Returns the number of elements the array can hold.
    ///
    /// # Examples
    ///
    /// ```
    /// use stack_array::*;
    ///
    /// let arr: ArrayBuf<u8, 4> = ArrayBuf::new();
    /// assert_eq!(arr.capacity(), 4);
    /// ```
    fn capacity(&self) -> usize;

    /// Shortens the array, keeping the first `len` elements and dropping
    /// the rest.
    ///
    /// If `len` is greater than the array's current length, this has no
    /// effect.
    fn truncate(&mut self, len: usize) {
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
            if len > self.len() {
                return;
            }
            let remaining_len = self.len() - len;
            let s = ptr::slice_from_raw_parts_mut(self.as_mut_ptr().add(len), remaining_len);
            self.set_len(len);
            ptr::drop_in_place(s);
        }
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
    fn as_ptr(&self) -> *const T;

    /// Returns an unsafe mutable pointer to the array's buffer.
    ///
    /// The caller must ensure that the array outlives the pointer this
    /// function returns, or else it will end up pointing to garbage.
    /// Modifying the array may cause its buffer to be reallocated,
    /// which would also make any pointers to it invalid.
    fn as_mut_ptr(&mut self) -> *mut T;

    /// Forces the length of the array to `new_len`.
    ///
    /// This is a low-level operation that maintains none of the normal
    /// invariants of the type. Normally changing the length of a array
    /// is done using one of the safe operations instead, such as
    /// [`truncate`] or [`clear`].
    ///
    /// [`truncate`]: Array::truncate
    /// [`clear`]: Array::clear
    ///
    /// # Safety
    ///
    /// - `new_len` must be less than or equal to [`capacity()`].
    /// - The elements at `old_len..new_len` must be initialized.
    ///
    /// [`capacity()`]: Vec::capacity
    unsafe fn set_len(&mut self, len: usize);

    /// Extracts a slice containing the entire array.
    ///
    /// Equivalent to `&s[..]`.
    #[inline]
    fn as_slice(&self) -> &[T] {
        // SAFETY: slice will contain only initialized objects.
        unsafe { slice::from_raw_parts(self.as_ptr(), self.len()) }
    }

    /// Extracts a mutable slice of the entire array.
    ///
    /// Equivalent to `&mut s[..]`.
    #[inline]
    fn as_mut_slice(&mut self) -> &mut [T] {
        // SAFETY: slice will contain only initialized objects.
        unsafe { slice::from_raw_parts_mut(self.as_mut_ptr(), self.len()) }
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
    /// use stack_array::*;
    ///
    /// let mut arr = ArrayBuf::from(["foo", "bar", "baz", "qux"]);
    ///
    /// assert_eq!(arr.swap_remove(1), "bar");
    /// assert_eq!(arr[..], ["foo", "qux", "baz"]);
    ///
    /// assert_eq!(arr.swap_remove(0), "foo");
    /// assert_eq!(arr[..], ["baz", "qux"]);
    /// ```
    #[inline]
    fn swap_remove(&mut self, index: usize) -> T {
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
    /// use stack_array::*;
    ///
    /// let mut list: ArrayBuf<u8, 3> = ArrayBuf::from([3].as_ref());
    /// list.insert(0, 1);
    /// assert_eq!(&list[..], [1, 3]);
    /// list.insert(1, 2);
    /// assert_eq!(&list, &[1, 2, 3]);
    /// ```
    ///
    /// # Panics
    /// Panics if the index is out of bounds.
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
        let total_len = len + 1;
        self.ensure_capacity(total_len);

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
            self.set_len(total_len);
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
    /// use stack_array::*;
    ///
    /// let mut list = ArrayBuf::from([1, 2, 3]);
    /// assert_eq!(list.remove(0), 1);
    /// assert_eq!(list.remove(0), 2);
    /// assert_eq!(list.remove(0), 3);
    /// ```
    ///
    /// # Panics
    /// Panics if the index is out of bounds.
    fn remove(&mut self, index: usize) -> T {
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
    /// use stack_array::*;
    ///
    /// let mut arr = ArrayBuf::from([1, 2, 3, 4]);
    ///
    /// arr.retain(|x| *x % 2 == 0);
    /// assert_eq!(arr[..], [2, 4]);
    /// ```
    ///
    /// Because the elements are visited exactly once in the original order,
    /// external state may be used to decide which elements to keep.
    ///
    /// ```
    /// use stack_array::*;
    ///
    /// let mut arr  = ArrayBuf::from([1, 2, 3, 4, 5]);
    /// let keep = [false, true, true, false, true];
    /// let mut iter = keep.iter();
    /// arr.retain(|_| *iter.next().unwrap());
    /// assert_eq!(arr[..], [2, 3, 5]);
    /// ```
    fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&T) -> bool,
    {
        retain_mut(self, |elem| f(elem))
    }

    fn drain<R>(&mut self, range: R) -> Drain<'_, T, Self>
    where
        R: RangeBounds<usize>,
    {
        let len = self.len();
        let Range { start, end } = slice_range(range, ..len);

        unsafe {
            // set self.vec length's to start, to be safe in case Drain is leaked
            self.set_len(start);
            // Use the borrow in the IterMut to indicate borrowing behavior of the
            // whole Drain iterator (like &mut T).
            let range_slice = slice::from_raw_parts_mut(self.as_mut_ptr().add(start), end - start);
            Drain {
                tail_start: end,
                tail_len: len - end,
                iter: range_slice.iter(),
                vec: ptr::NonNull::from(self),
            }
        }
    }

    #[inline]
    fn dedup(&mut self)
    where
        T: PartialEq,
    {
        self.dedup_by(|a, b| a == b)
    }

    /// Removes all but the first of consecutive elements in the array that resolve to the same
    /// key.
    ///
    /// If the array is sorted, this removes all duplicates.
    ///
    /// # Examples
    ///
    /// ```
    /// use stack_array::*;
    ///
    /// let mut arr = ArrayBuf::from([10, 20, 21, 30, 20]);
    ///
    /// arr.dedup_by_key(|i| *i / 10);
    ///
    /// assert_eq!(arr[..], [10, 20, 30, 20]);
    fn dedup_by_key<F, K>(&mut self, mut key: F)
    where
        F: FnMut(&mut T) -> K,
        K: PartialEq,
    {
        self.dedup_by(|a, b| key(a) == key(b))
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
        struct FillGapOnDrop<'a, T, A: Array<T>> {
            /* Offset of the element we want to check if it is duplicate */
            read: usize,

            /* Offset of the place where we want to place the non-duplicate
             * when we find it. */
            write: usize,

            /* The Vec that would need correction if `same_bucket` panicked */
            vec: &'a mut A,
            _marker: core::marker::PhantomData<T>,
        }

        impl<'a, T, A: Array<T>> Drop for FillGapOnDrop<'a, T, A> {
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
            _marker: core::marker::PhantomData,
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
        // This will panic or abort if we would allocate > isize::MAX bytes
        // or if the length increment would overflow for zero-sized types.
        let len = self.len();
        let total_len = len + 1;
        self.ensure_capacity(total_len);

        unsafe {
            let end = self.as_mut_ptr().add(len);
            ptr::write(end, value);
            self.set_len(total_len);
        }
    }

    #[inline]
    fn append(&mut self, other: &mut Self) {
        unsafe {
            let count = other.len();
            let len = self.len();
            let total_len = len + count;

            self.ensure_capacity(total_len);

            ptr::copy_nonoverlapping(
                other.as_ptr() as *const T,
                self.as_mut_ptr().add(len),
                count,
            );
            self.set_len(total_len);
            other.set_len(0);
        }
    }

    /// Clears the array, removing all values.
    ///
    /// # Examples
    ///
    /// ```
    /// use stack_array::*;
    ///
    /// let mut list = ArrayBuf::from([1, 2, 3]);
    /// list.clear();
    /// assert!(list.is_empty());
    /// ```
    #[inline]
    fn clear(&mut self) {
        self.truncate(0)
    }

    /// Returns the number of elements currently in the array.
    ///
    /// # Examples
    ///
    /// ```
    /// use stack_array::*;
    ///
    /// let arr: ArrayBuf<u8, 3> = ArrayBuf::from([1, 2].as_ref());
    /// assert_eq!(arr.len(), 2);
    /// ```
    fn len(&self) -> usize;

    /// Returns true if the array contains no elements.
    ///
    /// # Examples
    ///
    /// ```
    /// use stack_array::*;
    ///
    /// let mut arr: ArrayBuf<u8, 2> = ArrayBuf::new();
    /// assert!(arr.is_empty());
    ///
    /// arr.push(1);
    /// assert!(!arr.is_empty());
    /// ```
    #[inline]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Removes the last element from a collection and returns it.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use stack_array::*;
    ///
    /// let mut arr: ArrayBuf<u8, 3> = ArrayBuf::from([1, 2].as_ref());
    /// assert_eq!(arr.pop(), Some(2));
    /// assert_eq!(arr.pop(), Some(1));
    /// assert!(arr.is_empty());
    /// ```
    #[inline]
    fn pop(&mut self) -> Option<T> {
        if self.is_empty() {
            None
        } else {
            unsafe {
                let len = self.len() - 1;
                self.set_len(len);
                Some(ptr::read(self.as_ptr().add(len)))
            }
        }
    }

    //============================================================

    fn ensure_capacity(&mut self, total_len: usize) {
        if total_len > self.capacity() {
            panic!(
                "Array is full, Max capacity: {}, But got: {total_len}",
                self.capacity()
            );
        }
    }

    /// Returns the number of elements can be inserted into the array.
    ///
    /// # Examples
    ///
    /// ```
    /// use stack_array::*;
    ///
    /// let arr: ArrayBuf<u8, 3> = ArrayBuf::from([1, 2].as_ref());
    /// assert_eq!(arr.remaining_capacity(), 1);
    /// ```
    #[inline]
    fn remaining_capacity(&self) -> usize {
        self.capacity() - self.len()
    }

    #[inline]
    fn extend_from_slice(&mut self, other: impl AsRef<[T]>)
    where
        T: Copy,
    {
        let other = other.as_ref();
        let count = other.len();
        let len = self.len();
        let total_len = len + count;
        self.ensure_capacity(total_len);
        unsafe {
            ptr::copy_nonoverlapping(other.as_ptr(), self.as_mut_ptr().add(len), count);
            self.set_len(total_len);
        }
    }
}
