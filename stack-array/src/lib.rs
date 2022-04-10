#[doc = include_str!("../README.md")]

mod drain;
mod interface;
mod partial_eq;
mod retain_mut;
mod vector;
mod write;

pub use drain::Drain;
pub use interface::Array;
use retain_mut::retain_mut;

use core::{
    borrow::*,
    fmt, hash, mem,
    mem::MaybeUninit,
    ops,
    ops::{Deref, DerefMut, Index, IndexMut, Range, RangeBounds},
    ptr,
    ptr::NonNull,
    slice,
    slice::SliceIndex,
};
use std::cmp;

/// A data structure for storing and manipulating fixed number of elements of a specific type.
pub struct ArrayBuf<T, const N: usize> {
    len: usize,
    buf: [MaybeUninit<T>; N],
}

impl<T, const N: usize> ArrayBuf<T, N> {
    /// Constructs a new, `ArrayBuf`
    ///
    /// # Examples
    ///
    /// ```
    /// use stack_array::*;
    ///
    /// let mut arr: ArrayBuf<u8, 64> = ArrayBuf::new();
    /// ```
    #[inline]
    pub const fn new() -> Self {
        Self {
            len: 0,
            buf: unsafe { MaybeUninit::uninit().assume_init() },
        }
    }

    /// Returns `true`, If the array is full.
    ///
    /// # Examples
    ///
    /// ```
    /// use stack_array::*;
    ///
    /// let arr = ArrayBuf::from([1, 2, 3]);
    /// assert!(arr.is_full());
    /// ```
    #[inline]
    pub const fn is_full(&self) -> bool {
        self.len >= N
    }
}

impl<T, const N: usize> Array<T> for ArrayBuf<T, N> {
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
    unsafe fn set_len(&mut self, new_len: usize) {
        debug_assert!(new_len <= self.capacity());
        self.len = new_len;
    }

    #[inline]
    fn len(&self) -> usize {
        self.len
    }
}

impl<T, const N: usize> Drop for ArrayBuf<T, N> {
    fn drop(&mut self) {
        self.clear();
    }
}

impl<T, const N: usize> Default for ArrayBuf<T, N> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> AsRef<[T]> for ArrayBuf<T, N> {
    #[inline]
    fn as_ref(&self) -> &[T] {
        self
    }
}

impl<T, const N: usize> AsMut<[T]> for ArrayBuf<T, N> {
    #[inline]
    fn as_mut(&mut self) -> &mut [T] {
        self
    }
}

impl<T, const N: usize> Deref for ArrayBuf<T, N> {
    type Target = [T];
    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<T, const N: usize> DerefMut for ArrayBuf<T, N> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

impl<T: Copy, const N: usize> From<&[T]> for ArrayBuf<T, N> {
    fn from(values: &[T]) -> Self {
        let mut array = Self::new();
        array.extend_from_slice(values);
        array
    }
}

impl<T: Copy, const N: usize> From<[T; N]> for ArrayBuf<T, N> {
    fn from(values: [T; N]) -> Self {
        let mut array = Self::new();
        array.extend_from_slice(values);
        array
    }
}

impl<T, I: SliceIndex<[T]>, const N: usize> Index<I> for ArrayBuf<T, N> {
    type Output = I::Output;
    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        Index::index(&**self, index)
    }
}

impl<T, I: SliceIndex<[T]>, const N: usize> IndexMut<I> for ArrayBuf<T, N> {
    #[inline]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        IndexMut::index_mut(&mut **self, index)
    }
}

impl<T: fmt::Debug, const N: usize> fmt::Debug for ArrayBuf<T, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T, const N: usize> Borrow<[T]> for ArrayBuf<T, N> {
    fn borrow(&self) -> &[T] {
        &self[..]
    }
}

impl<T, const N: usize> BorrowMut<[T]> for ArrayBuf<T, N> {
    fn borrow_mut(&mut self) -> &mut [T] {
        &mut self[..]
    }
}

/// Implements comparison of vectors, [lexicographically](core::cmp::Ord#lexicographical-comparison).
impl<T: PartialOrd, const N: usize> cmp::PartialOrd for ArrayBuf<T, N> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        PartialOrd::partial_cmp(&**self, &**other)
    }
}

impl<T: Eq, const N: usize> Eq for ArrayBuf<T, N> {}

/// Implements ordering of vectors, [lexicographically](core::cmp::Ord#lexicographical-comparison).
impl<T: Ord, const N: usize> cmp::Ord for ArrayBuf<T, N> {
    #[inline]
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        Ord::cmp(&**self, &**other)
    }
}

impl<T: hash::Hash, const N: usize> hash::Hash for ArrayBuf<T, N> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        hash::Hash::hash(&**self, state)
    }
}
