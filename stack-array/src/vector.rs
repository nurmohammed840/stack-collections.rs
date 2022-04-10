impl<T> crate::Array<T> for std::vec::Vec<T> {
    fn capacity(&self) -> usize {
        Vec::capacity(self)
    }

    fn as_ptr(&self) -> *const T {
        Vec::as_ptr(self)
    }

    fn as_mut_ptr(&mut self) -> *mut T {
        Vec::as_mut_ptr(self)
    }

    unsafe fn set_len(&mut self, len: usize) {
        Vec::set_len(self, len)
    }

    fn insert(&mut self, index: usize, element: T) {
        Vec::insert(self, index, element)
    }

    fn dedup_by<F>(&mut self, same_bucket: F)
    where
        F: FnMut(&mut T, &mut T) -> bool,
    {
        Vec::dedup_by(self, same_bucket)
    }

    fn push(&mut self, value: T) {
        Vec::push(self, value)
    }

    fn len(&self) -> usize {
        Vec::len(self)
    }

    fn pop(&mut self) -> Option<T> {
        Vec::pop(self)
    }

    fn truncate(&mut self, len: usize) {
        Vec::truncate(self, len)
    }

    fn as_slice(&self) -> &[T] {
        Vec::as_slice(self)
    }

    fn as_mut_slice(&mut self) -> &mut [T] {
        Vec::as_mut_slice(self)
    }

    fn swap_remove(&mut self, index: usize) -> T {
        Vec::swap_remove(self, index)
    }

    fn remove(&mut self, index: usize) -> T {
        Vec::remove(self, index)
    }

    fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&T) -> bool,
    {
        Vec::retain(self, f)
    }

    fn dedup_by_key<F, K>(&mut self, key: F)
    where
        F: FnMut(&mut T) -> K,
        K: PartialEq,
    {
        Vec::dedup_by_key(self, key)
    }

    fn append(&mut self, other: &mut Self) {
        Vec::append(self, other)
    }

    fn clear(&mut self) {
        Vec::clear(self)
    }

    fn is_empty(&self) -> bool {
        Vec::is_empty(self)
    }

    // =========================================================================

    fn ensure_capacity(&mut self, new_len: usize) {
        if new_len > self.capacity() {
            Vec::reserve(self, new_len - self.len())
        }
    }
}
