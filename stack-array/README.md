[Doc](https://docs.rs/stack-array/)

This library provides an array type that is similar to the built-in Vec type, but lives on the stack!

You may use this library to store a fixed number of elements of a specific type (even non-copy types)

# Example

```rust
use stack_array::*;

let mut arr: ArrayBuf<String, 4> = ArrayBuf::new();

arr.push("Hello".into());
arr.push("World".into());

println!("{:#?}", arr);
```

Note: Documentation is incomplete and may be inaccurate. I do not have the time to update it. Please report any issues, or contribute!