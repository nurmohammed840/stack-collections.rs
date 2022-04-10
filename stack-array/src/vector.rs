impl<T> crate::Array<T> for std::vec::Vec<T> {
    #[inline]
    fn capacity(&self) -> usize {
        Vec::capacity(self)
    }

    #[inline]
    fn truncate(&mut self, len: usize) {
        Vec::truncate(self, len)
    }

    #[inline]
    fn as_ptr(&self) -> *const T {
        Vec::as_ptr(self)
    }

    #[inline]
    fn as_mut_ptr(&mut self) -> *mut T {
        Vec::as_mut_ptr(self)
    }

    #[inline]
    unsafe fn set_len(&mut self, len: usize) {
        Vec::set_len(self, len)
    }

    #[inline]
    fn as_slice(&self) -> &[T] {
        Vec::as_slice(self)
    }

    #[inline]
    fn as_mut_slice(&mut self) -> &mut [T] {
        Vec::as_mut_slice(self)
    }

    #[inline]
    fn swap_remove(&mut self, index: usize) -> T {
        Vec::swap_remove(self, index)
    }

    #[inline]
    fn insert(&mut self, index: usize, element: T) {
        Vec::insert(self, index, element)
    }

    #[inline]
    fn remove(&mut self, index: usize) -> T {
        Vec::remove(self, index)
    }

    #[inline]
    fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&T) -> bool,
    {
        Vec::retain(self, f)
    }

    #[inline]
    fn dedup(&mut self)
    where
        T: PartialEq,
    {
        Vec::dedup(self)
    }

    #[inline]
    fn dedup_by_key<F, K>(&mut self, key: F)
    where
        F: FnMut(&mut T) -> K,
        K: PartialEq,
    {
        Vec::dedup_by_key(self, key)
    }

    #[inline]
    fn dedup_by<F>(&mut self, same_bucket: F)
    where
        F: FnMut(&mut T, &mut T) -> bool,
    {
        Vec::dedup_by(self, same_bucket)
    }

    #[inline]
    fn push(&mut self, value: T) {
        Vec::push(self, value)
    }

    #[inline]
    fn append(&mut self, other: &mut Self) {
        Vec::append(self, other)
    }

    #[inline]
    fn clear(&mut self) {
        Vec::clear(self)
    }

    #[inline]
    fn len(&self) -> usize {
        Vec::len(self)
    }

    #[inline]
    fn is_empty(&self) -> bool {
        Vec::is_empty(self)
    }

    #[inline]
    fn pop(&mut self) -> Option<T> {
        Vec::pop(self)
    }

    // =========================================================================
    #[inline]
    fn ensure_capacity(&mut self, new_len: usize) {
        if new_len > self.capacity() {
            Vec::reserve(self, new_len - self.len())
        }
    }
}
