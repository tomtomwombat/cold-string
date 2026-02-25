# cold-string
[![Github](https://img.shields.io/badge/github-8da0cb?style=for-the-badge&labelColor=555555&logo=github)](https://github.com/tomtomwombat/cold-string)
[![Crates.io](https://img.shields.io/badge/crates.io-fc8d62?style=for-the-badge&labelColor=555555&logo=rust)](https://crates.io/crates/cold-string)
[![docs.rs](https://img.shields.io/badge/docs.rs-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs)](https://docs.rs/cold-string)
![MSRV](https://img.shields.io/crates/msrv/cold-string?style=for-the-badge)
![Downloads](https://img.shields.io/crates/d/cold-string?style=for-the-badge)

A 1-word sized representation of immutable UTF-8 strings. In-lines up to 1 word bytes. Optimized for memory usage and struct packing.

# Usage

Use it like a `String`:
```rust
use cold_string::ColdString;

let s = ColdString::new("qwerty");
assert_eq!(s.as_str(), "qwerty");
```

Packs well with other types:
```rust
use std::mem;
use cold_string::ColdString;

assert_eq!(mem::size_of::<ColdString>(), mem::size_of::<usize>());
assert_eq!(mem::align_of::<ColdString>(), 1);

assert_eq!(mem::size_of::<(ColdString, u8)>(), mem::size_of::<usize>() + 1);
assert_eq!(mem::align_of::<(ColdString, u8)>(), 1);
```

# How It Works

ColdString is 8-byte tagged pointer (4 bytes on 32-bit machines):
```rust
#[repr(packed)]
pub struct ColdString {
    /// The first byte of `encoded` is the "tag" and it determines the type:
    /// - 10xxxxxx: an encoded address for the heap. To decode, 10 is set to 00 and swapped
    ///   with the LSB bits of the tag byte. The address is always a multiple of 4 (`HEAP_ALIGN`).
    /// - 11111xxx: xxx is the length in range 0..=7, followed by length UTF-8 bytes.
    /// - xxxxxxxx (valid UTF-8): 8 UTF-8 bytes.
    encoded: *mut u8,
}
```
`encoded` acts as either a pointer to the heap for strings longer than 8 bytes or is the inlined data itself. The first/"tag" byte indicates one of 3 encodings:

## Inline Mode (0 to 7 Bytes)
The tag byte has bits 11111xxx, where xxx is the length. `self.0[1]` to `self.0[7]` store the bytes of string.

## Inline Mode (8 Bytes)
The tag byte is any valid UTF-8 byte. `self.0` stores the bytes of string. Since the string is UTF-8, the tag byte is guaranteed to not be 10xxxxx or 11111xxx.

## Heap Mode
`self.0` encodes the pointer to heap, where tag byte is 10xxxxxx. 10xxxxxx is chosen because it's a UTF-8 continuation byte and therefore an impossible tag byte for inline mode. Since a heap-alignment of 4 is chosen, the pointer's least significant 2 bits are guaranteed to be 0 ([See more](https://doc.rust-lang.org/beta/std/alloc/struct.Layout.html#method.from_size_align)). These bits are swapped with the 10 "tag" bits when de/coding between `self.0` and the address value.

On the heap, the data starts with a variable length integer encoding of the length, followed by the bytes.
```text,ignore
ptr --> <var int length> <data>
```

# Memory Comparisons

![string_memory](https://github.com/user-attachments/assets/6644ae40-1da7-42e2-9ae6-0596e77e953e)

## Memory Usage Comparison (RSS per String)

| Crate | 0–4 chars | 0–8 chars | 0–16 chars | 0–32 chars | 0–64 chars |
| :--- | :---: | :---: | :---: | :---: | :---: |
| `std` | 36.9 B | 38.4 B | 46.8 B | 55.3 B | 71.4 B |
| `smol_str` | 24.0 B | 24.0 B | 24.0 B | 41.1 B | 72.2 B |
| `compact_str` | 24.0 B | 24.0 B | 24.0 B | 35.4 B | 61.0 B |
| `compact_string` | 24.1 B | 25.8 B | 32.6 B | 40.5 B | 56.5 B |
| **`cold-string`** | **8.0 B** | **8.0 B** | **23.2 B** | **35.7 B** | **53.0 B** |

**Note:** Columns represent string length (bytes/chars). Values represent average Resident Set Size (RSS) in bytes per string instance. Measurements taken with 10M iterations.

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
