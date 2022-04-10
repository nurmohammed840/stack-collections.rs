use crate::*;

macro_rules! __impl_slice_eq1 {
    ([$($vars:tt)*] $lhs:ty, $rhs:ty $(where $ty:ty: $bound:ident)?) => {
        impl<T, U, $($vars)*> PartialEq<$rhs> for $lhs
        where
            T: PartialEq<U>,
            $($ty: $bound)?
        {
            #[inline]
            fn eq(&self, other: &$rhs) -> bool { self[..] == other[..] }
        }
    }
}

__impl_slice_eq1! { [const N: usize] ArrayBuf<T, N>, ArrayBuf<U, N>}
__impl_slice_eq1! { [const N: usize] ArrayBuf<T, N>, &[U]}
__impl_slice_eq1! { [const N: usize] ArrayBuf<T, N>, &mut [U]}
__impl_slice_eq1! { [const N: usize] &[T], ArrayBuf<U, N>}
__impl_slice_eq1! { [const N: usize] &mut [T], ArrayBuf<U, N>}
__impl_slice_eq1! { [const N: usize] ArrayBuf<T, N>, [U] }
__impl_slice_eq1! { [const N: usize] [T], ArrayBuf<U, N> }

__impl_slice_eq1! { [const N: usize] ArrayBuf<T, N>, [U; N]}
__impl_slice_eq1! { [const N: usize] ArrayBuf<T, N>, &[U; N]}
