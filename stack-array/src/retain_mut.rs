use crate::*;

pub fn retain_mut<F, Arr: Array<T>, T>(this: &mut Arr, mut f: F)
where
    F: FnMut(&mut T) -> bool,
{
    let original_len = this.len();
    // Avoid double drop if the drop guard is not executed,
    // since we may make some holes during the process.
    unsafe { this.set_len(0) };

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
    struct BackshiftOnDrop<'a, Arr: Array<T>, T> {
        v: &'a mut Arr,
        processed_len: usize,
        deleted_cnt: usize,
        original_len: usize,
        _marker: core::marker::PhantomData<T>,
    }

    impl<Arr: Array<T>, T> Drop for BackshiftOnDrop<'_, Arr, T> {
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
        v: this,
        processed_len: 0,
        deleted_cnt: 0,
        original_len,
        _marker: core::marker::PhantomData,
    };

    fn process_loop<F, Arr: Array<T>, T, const DELETED: bool>(
        original_len: usize,
        f: &mut F,
        g: &mut BackshiftOnDrop<'_, Arr, T>,
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
    process_loop::<F, Arr, T, false>(original_len, &mut f, &mut g);

    // Stage 2: Some elements were deleted.
    process_loop::<F, Arr, T, true>(original_len, &mut f, &mut g);

    // All item are processed. This can be optimized to `set_len` by LLVM.
    drop(g);
}
