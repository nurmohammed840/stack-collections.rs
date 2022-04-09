use stack_array::{ArrayBuf, Array};
use std::mem::size_of;

struct DropCounter<'a> {
    count: &'a mut u32,
}

impl Drop for DropCounter<'_> {
    fn drop(&mut self) {
        *self.count += 1;
    }
}

#[test]
fn test_small_vec_struct() {
    assert_eq!(size_of::<ArrayBuf<u8, 8>>(), 16);
}

#[test]
fn test_double_drop() {
    struct TwoArray<T> {
        x: ArrayBuf<T, 2>,
        y: ArrayBuf<T, 3>,
    }

    let (mut count_x, mut count_y) = (0, 0);
    {
        let mut tv = TwoArray {
            x: ArrayBuf::new(),
            y: ArrayBuf::new(),
        };
        tv.x.push(DropCounter {
            count: &mut count_x,
        });
        tv.y.push(DropCounter {
            count: &mut count_y,
        });

        // If Vec had a drop flag, here is where it would be zeroed.
        // Instead, it should rely on its internal state to prevent
        // doing anything significant when dropped multiple times.
        drop(tv.x);

        // Here tv goes out of scope, tv.y should be dropped, but not tv.x.
    }

    assert_eq!(count_x, 1);
    assert_eq!(count_y, 1);
}

#[test]
fn test_indexing() {
    let v: ArrayBuf<isize, 2> = ArrayBuf::from([10, 20].as_slice());
    assert_eq!(v[0], 10);
    assert_eq!(v[1], 20);
    let mut x: usize = 0;
    assert_eq!(v[x], 10);
    assert_eq!(v[x + 1], 20);
    x = x + 1;
    assert_eq!(v[x], 20);
    assert_eq!(v[x - 1], 10);
}

#[test]
fn test_debug_fmt() {
    let arr1: ArrayBuf<isize, 2> = ArrayBuf::new();
    assert_eq!("[]", format!("{:?}", arr1));

    let vec2: ArrayBuf<_, 2> = ArrayBuf::from([0, 1].as_slice());
    assert_eq!("[0, 1]", format!("{:?}", vec2));

    let slice: &[isize] = &[4, 5];
    assert_eq!("[4, 5]", format!("{:?}", slice));
}

#[test]
fn test_push() {
    let mut v: ArrayBuf<_, 3> = ArrayBuf::new();
    v.push(1);
    assert_eq!(v.as_slice(), [1]);
    v.push(2);
    assert_eq!(v.as_slice(), [1, 2]);
    v.push(3);
    assert_eq!(v.as_slice(), [1, 2, 3]);
}

#[test]
fn test_slice_from_ref() {
    let values: ArrayBuf<_, 5> = [1, 2, 3, 4, 5].as_slice().into();
    let slice = &values[1..3];

    assert_eq!(slice, [2, 3]);
}

#[test]
fn test_slice_from_mut() {
    let mut values: ArrayBuf<_, 5> = [1, 2, 3, 4, 5].as_slice().into();
    {
        let slice = &mut values[2..];
        assert!(slice == [3, 4, 5]);
        for p in slice {
            *p += 2;
        }
    }

    assert!(values.as_slice() == [1, 2, 5, 6, 7]);
}

#[test]
fn test_slice_to_mut() {
    let mut values: ArrayBuf<_, 5> = [1, 2, 3, 4, 5].as_slice().into();
    {
        let slice = &mut values[..2];
        assert!(slice == [1, 2]);
        for p in slice {
            *p += 1;
        }
    }
    assert!(values.as_slice() == [2, 3, 3, 4, 5]);
}

#[test]
fn test_split_at_mut() {
    let mut values: ArrayBuf<_, 5> = [1, 2, 3, 4, 5].as_slice().into();
    {
        let (left, right) = values.split_at_mut(2);
        {
            let left: &[_] = left;
            assert!(&left[..left.len()] == &[1, 2]);
        }
        for p in left {
            *p += 1;
        }

        {
            let right: &[_] = right;
            assert!(&right[..right.len()] == &[3, 4, 5]);
        }
        for p in right {
            *p += 2;
        }
    }
    assert_eq!(values.as_slice(), [2, 3, 5, 6, 7]);
}

#[test]
fn test_clone() {
    let v: Vec<i32> = vec![];
    let w = vec![1, 2, 3];

    assert_eq!(v, v.clone());

    let z = w.clone();
    assert_eq!(w, z);
    // they should be disjoint in memory.
    assert!(w.as_ptr() != z.as_ptr())
}

#[test]
fn test_retain() {
    let mut arr: ArrayBuf<_, 4> = [1, 2, 3, 4].as_slice().into();
    arr.retain(|&x| x % 2 == 0);
    assert_eq!(arr.as_ref(), [2, 4]);
}

type Arr = ArrayBuf<u8, 10>;
macro_rules! arr {
    () => (
        Arr::new()
    );
    ($($x:expr),+ $(,)?) => (
        Arr::from([$($x),+].as_slice())
    );
}

#[test]
fn test_dedup_by_key() {
    fn case(a: Arr, b: Arr) {
        let mut v = a;
        v.dedup_by_key(|i| *i / 10);
        assert_eq!(v.as_slice(), b.as_slice());
    }
    case(arr![], arr![]);
    case(arr![10], arr![10]);
    case(arr![10, 11], arr![10]);
    case(arr![10, 20, 30], arr![10, 20, 30]);
    case(arr![10, 11, 20, 30], arr![10, 20, 30]);
    case(arr![10, 20, 21, 30], arr![10, 20, 30]);
    case(arr![10, 20, 30, 31], arr![10, 20, 30]);
    case(arr![10, 11, 20, 21, 22, 30, 31], arr![10, 20, 30]);
}
