#![no_std]

use core::{
    fmt,
    mem::{replace, MaybeUninit},
    ops::{Deref, DerefMut},
    ptr,
};

/// SEAFTY: Caller must ensure that `value` is properly initialized.
unsafe fn take<T>(dest: &mut MaybeUninit<T>) -> T {
    replace(dest, MaybeUninit::uninit()).assume_init()
}

pub struct Array<T, const N: usize> {
    len: usize,
    data: [MaybeUninit<T>; N],
}

impl<T, const N: usize> Array<T, N> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn capacity(&self) -> usize {
        N
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn remaining(&self) -> usize {
        N - self.len
    }

    pub fn push(&mut self, val: T) {
        self.data[self.len] = MaybeUninit::new(val);
        self.len += 1;
    }

    pub fn pop(&mut self) -> T {
        self.len -= 1;
        unsafe { take(&mut self.data[self.len]) }
    }

    pub fn is_full(&self) -> bool {
        self.len >= N
    }

    pub fn clear(&mut self) {
        // SAFETY: mutable slice will contain only initialized objects, So It's safe to drop them.
        unsafe {
            ptr::drop_in_place(self.as_mut());
        }
        self.len = 0;
    }

    pub fn insert(&mut self, index: usize, val: T) {
        assert!(index <= self.len);
        for i in (index..self.len).rev() {
            self.data[i + 1] = replace(&mut self.data[i], MaybeUninit::uninit());
        }
        self.data[index] = MaybeUninit::new(val);
        self.len += 1;
    }

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
        Self {
            len: 0,
            data: unsafe { MaybeUninit::uninit().assume_init() },
        }
    }
}

impl<T, const N: usize> AsRef<[T]> for Array<T, N> {
    fn as_ref(&self) -> &[T] {
        // unsafe { core::mem::transmute(&self.data[..self.len]) }
        // unsafe { slice::from_raw_parts(self.data.as_ptr() as *const _, self.len) }

        // SAFETY: `self.data[..self.len]` is initialized.
        unsafe { &*(&self.data[..self.len] as *const [MaybeUninit<T>] as *const [T]) }
    }
}

impl<T, const N: usize> AsMut<[T]> for Array<T, N> {
    fn as_mut(&mut self) -> &mut [T] {
        // SAFETY: `self.data[..self.len]` is initialized.
        unsafe { &mut *(&mut self.data[..self.len] as *mut [MaybeUninit<T>] as *mut [T]) }
    }
}

impl<T, const N: usize> Deref for Array<T, N> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<T, const N: usize> DerefMut for Array<T, N> {
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn push() {
        let mut list: Array<u8, 3> = Array::from([1]);
        list.push(2);
        list.push(3);
        assert_eq!(&list[..], [1, 2, 3]);
    }

    #[test]
    fn pop() {
        let mut list: Array<u8, 3> = Array::from([1, 2]);
        assert_eq!(list.pop(), 2);
        assert_eq!(list.pop(), 1);
        assert_eq!(list.len(), 0);
    }

    #[test]
    fn remove() {
        let mut list: Array<u8, 3> = Array::from([1, 2, 3]);
        assert_eq!(list.remove(0), 1);
        assert_eq!(list.remove(0), 2);
        assert_eq!(list.remove(0), 3);
    }

    #[test]
    fn insert() {
        let mut list: Array<u8, 3> = Array::from([3]);
        list.insert(0, 1);
        assert_eq!(&list[..], [1, 3]);
        list.insert(1, 2);
        assert_eq!(&list[..], [1, 2, 3]);
    }

    #[test]
    fn list_from_buf() {
        assert_eq!(Array::<u8, 3>::new().len(), 0);
        assert_eq!(Array::<u8, 3>::from([1, 2]).len(), 2);
    }

    #[test]
    fn list_from_slice() {
        assert_eq!(Array::<u8, 3>::new().len(), 0);
        assert_eq!(Array::<u8, 3>::from([1, 2].as_ref()).len(), 2);
    }
}
