#![allow(warnings)]

// Work in progress...

use array::Array;

#[derive(Default)]
struct BtreeMap<K , V, const N: usize> {
    len: usize,
    nodes: Array<Node<K, V>, N>,
}

struct Node<K, V> {
    key: K,
    value: V,
    left: Option<usize>,
    right: Option<usize>,
}



