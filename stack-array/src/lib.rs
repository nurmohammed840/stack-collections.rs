#![no_std]

//! This library provides an array type that is similar to the built-in Vec type, but lives on the stack!
//!
//! You can store a fixed number of elements of a specific type (even non-copy types!)

use core::{
    fmt,
    mem::{replace, MaybeUninit},
    ops::{Deref, DerefMut},
    ptr,
};

/// Caller must ensure that `value` is properly initialized.
unsafe fn take<T>(dest: &mut MaybeUninit<T>) -> T {
    replace(dest, MaybeUninit::uninit()).assume_init()
}

/// A data structure for storing and manipulating fixed number of elements of a specific type.
pub struct Array<T, const N: usize> {
    len: usize,
    data: [MaybeUninit<T>; N],
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
    pub const fn new() -> Self {
        Self {
            len: 0,
            // SAFETY: An uninitialized `[MaybeUninit<_>; N]` is valid.
            data: unsafe { MaybeUninit::uninit().assume_init() },
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
    #[inline(always)]
    pub const fn capacity(&self) -> usize {
        N
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
    #[inline(always)]
    pub const fn len(&self) -> usize {
        self.len
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
    #[inline(always)]
    pub const fn is_full(&self) -> bool {
        N == self.len
    }

    /// Returns true if the vector contains no elements.
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
    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
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
    #[inline(always)]
    pub const fn remaining_capacity(&self) -> usize {
        N - self.len
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
    #[inline(always)]
    pub fn push(&mut self, val: T) {
        self.data[self.len] = MaybeUninit::new(val);
        self.len += 1;
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
    #[inline(always)]
    pub fn pop(&mut self) -> T {
        self.len -= 1;
        unsafe { take(&mut self.data[self.len]) }
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
    pub fn clear(&mut self) {
        // SAFETY: slice will contain only initialized objects, So It's safe to drop them.
        for slot in &mut self.data[0..self.len] {
            unsafe {
                ptr::drop_in_place(slot.as_mut_ptr());
            }
        }
        self.len = 0;
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
    #[inline]
    pub fn insert(&mut self, index: usize, val: T) {
        assert!(index <= self.len);
        for i in (index..self.len).rev() {
            self.data[i + 1] = replace(&mut self.data[i], MaybeUninit::uninit());
        }
        self.data[index] = MaybeUninit::new(val);
        self.len += 1;
    }

    /// Removes an element from position index within the array, shifting all elements after it to the left.
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
    #[inline]
    pub fn remove(&mut self, index: usize) -> T {
        assert!(index < self.len);
        let value = unsafe { take(&mut self.data[index]) };
        self.len -= 1;
        for i in index..self.len {
            self.data[i] = replace(&mut self.data[i + 1], MaybeUninit::uninit());
        }
        value
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
        // unsafe { core::mem::transmute(&self.data[..self.len]) }
        // unsafe { slice::from_raw_parts(self.data.as_ptr() as *const _, self.len) }

        // SAFETY: slice will contain only initialized objects.
        unsafe { &*(&self.data[..self.len] as *const [MaybeUninit<T>] as *const [T]) }
    }
}

impl<T, const N: usize> AsMut<[T]> for Array<T, N> {
    #[inline]
    fn as_mut(&mut self) -> &mut [T] {
        // SAFETY: slice will contain only initialized objects.
        unsafe { &mut *(&mut self.data[..self.len] as *mut [MaybeUninit<T>] as *mut [T]) }
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
            .field("data", &self.as_ref())
            .finish()
    }
}

impl<T: Clone, const N: usize> From<&[T]> for Array<T, N> {
    fn from(values: &[T]) -> Self {
        let mut array = Self::default();
        for val in values {
            array.push(val.clone());
        }
        array
    }
}

impl<T, const N: usize, const S: usize> From<[T; S]> for Array<T, N> {
    fn from(values: [T; S]) -> Self {
        let mut array = Self::default();
        for val in values {
            array.push(val);
        }
        array
    }
}
