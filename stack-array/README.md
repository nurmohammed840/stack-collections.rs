[Doc](https://docs.rs/stack-array/)

This library provides an array type that is similar to the built-in Vec type, but lives on the stack!

You may use this library to store a fixed number of elements of a specific type (even non-copy types!)

# Example

```rust
use stack_array::Array;

let mut arr: Array<String; 4> = Array::new();

arr.push("Hello".to_string());
arr.push("World".to_string());

println!("{:#?}", arr);
```