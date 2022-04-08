use crate::*;

pub trait ArrayInterface<T>: AsRef<[T]> + AsMut<[T]> + Default {
    /// Constructs a new, empty `Vec<T>`.
    ///
    /// The array will not allocate until elements are pushed onto it.
    ///
    /// # Examples
    ///
    /// ```
    /// use stack_array::*;
    ///
    /// let mut arr: Array<u8, 64> = Array::new();
    /// ```
    fn new() -> Self;

    /// Returns the number of elements the array can hold.
    ///
    /// # Examples
    ///
    /// ```
    /// use stack_array::*;
    ///
    /// let arr: Array<u8, 4> = Array::new();
    /// assert_eq!(arr.capacity(), 4);
    /// ```
    fn capacity(&self) -> usize;

    /// Shortens the array, keeping the first `len` elements and dropping
    /// the rest.
    ///
    /// If `len` is greater than the array's current length, this has no
    /// effect.
    ///
    /// The [`drain`] method can emulate `truncate`, but causes the excess
    /// elements to be returned instead of dropped.
    ///
    /// Note that this method has no effect on the allocated capacity
    /// of the array.
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

    /// Extracts a slice containing the entire array.
    ///
    /// Equivalent to `&s[..]`.
    #[inline]
    fn as_slice(&self) -> &[T] {
        self.as_ref()
    }

    /// Extracts a mutable slice of the entire array.
    ///
    /// Equivalent to `&mut s[..]`.
    #[inline]
    fn as_mut_slice(&mut self) -> &mut [T] {
        self.as_mut()
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
    /// let mut arr: Array<&str, 4> = Array::from(["foo", "bar", "baz", "qux"]);
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
    /// let mut list: Array<u8, 3> = Array::from([3]);
    /// list.insert(0, 1);
    /// assert_eq!(&list[..], [1, 3]);
    /// list.insert(1, 2);
    /// assert_eq!(&list[..], [1, 2, 3]);
    /// ```
    ///
    /// # Panics
    /// Panics if the index is out of bounds.
    fn insert(&mut self, index: usize, element: T);

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
    /// let mut list: Array<u8, 3> = Array::from([1, 2, 3]);
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
    /// use stack_array::*;
    ///
    /// let mut arr: Array<u8, 5> = Array::from([1, 2, 3, 4, 5]);
    /// let keep = [false, true, true, false, true];
    /// let mut iter = keep.iter();
    /// arr.retain(|_| *iter.next().unwrap());
    /// assert_eq!(arr[..], [2, 3, 5]);
    /// ```
    fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&T) -> bool,
    {
        self.retain_mut(|elem| f(elem));
    }

    fn retain_mut<F>(&mut self, f: F)
    where
        F: FnMut(&mut T) -> bool;

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
    /// let mut arr: Array<u8, 5> = Array::from([10, 20, 21, 30, 20]);
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

    fn dedup_by<F>(&mut self, same_bucket: F)
    where
        F: FnMut(&mut T, &mut T) -> bool;

    fn push(&mut self, value: T);

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
    /// let mut list: Array<u8, 3> = Array::from([1, 2, 3]);
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
    /// let arr: Array<u8, 3> = Array::from([1, 2]);
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
    /// let mut arr: Array<u8, 2> = Array::new();
    /// assert!(arr.is_empty());
    ///
    /// arr.push(1);
    /// assert!(!arr.is_empty());
    /// ```
    #[inline]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    //============================================================

    fn ensure_capacity(&mut self, total_cap: usize) {
        if total_cap > self.capacity() {
            panic!(
                "Array is full, Max capacity: {}, But got: {total_cap}",
                self.capacity()
            );
        }
    }

    /// Removes the last element from a collection and returns it.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use stack_array::*;
    ///
    /// let mut arr: Array<u8, 3> = Array::from([1, 2]);
    /// assert_eq!(arr.pop(), 2);
    /// assert_eq!(arr.pop(), 1);
    /// assert!(arr.is_empty());
    /// ```
    ///
    /// # Panics
    /// Panics if the array is empty.
    fn pop(&mut self) -> T;

    /// Returns the number of elements can be inserted into the array.
    ///
    /// # Examples
    ///
    /// ```
    /// use stack_array::*;
    ///
    /// let arr: Array<u8, 3> = Array::from([1, 2]);
    /// assert_eq!(arr.remaining_capacity(), 1);
    /// ```
    #[inline]
    fn remaining_capacity(&self) -> usize {
        self.capacity() - self.len()
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
    /// use stack_array::*;
    ///
    /// let mut arr: Array<u8, 6> = Array::from([1, 2, 3]);
    /// arr.append_slice([4, 5, 6]);
    /// assert_eq!(arr[..], [1, 2, 3, 4, 5, 6]);
    /// ```
    #[inline]
    fn append_slice(&mut self, other: impl AsRef<[T]>)
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
