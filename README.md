# cold-string
[![Github](https://img.shields.io/badge/github-8da0cb?style=for-the-badge&labelColor=555555&logo=github)](https://github.com/tomtomwombat/cold-string)
[![Crates.io](https://img.shields.io/badge/crates.io-fc8d62?style=for-the-badge&labelColor=555555&logo=rust)](https://crates.io/crates/cold-string)
[![docs.rs](https://img.shields.io/badge/docs.rs-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs)](https://docs.rs/cold-string)
![Downloads](https://img.shields.io/crates/d/cold-string?style=for-the-badge)

Compact representation of immutable UTF-8 strings. Optimized for memory usage and struct packing.

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

assert_eq!(mem::size_of::<ColdString>(), 8);
assert_eq!(mem::align_of::<ColdString>(), 1);

assert_eq!(mem::size_of::<(ColdString, u8)>(), 9);
assert_eq!(mem::align_of::<(ColdString, u8)>(), 1);
```

# How It Works

ColdString is an 8 byte array (4 bytes on 32-bit machines):
```rust,ignore
pub struct ColdString([u8; 8]);
```
The array acts as either a pointer to heap data for strings longer than 7 bytes or is the inlined data itself.
## Inline Mode
`self.0[1]` to `self.0[7]` store the bytes of string. In the least significant byte, `self.0[0]`, the least significant bit signifies the inline/heap flag, and is set to "1" for inline mode. The next bits encode the length (always between 0 and 7).
```text,ignore
b0 b1 b2 b3 b4 b5 b6 b7
b0 = <7 bit len> | 1
```
For example, `"qwerty" = [13, 'q', 'w', 'e', 'r', 't', 'y', 0]`, where 13 is `"qwerty".len() << 1 | 1`.

## Heap Mode
The bytes act as a pointer to heap. The data on the heap has alignment 2, causing the least significant bit to always be 0 (since alignment 2 implies `addr % 2 == 0`), signifying heap mode. On the heap, the data starts with a variable length integer encoding of the length, followed by the bytes.
```text,ignore
ptr --> <var int length> <data>
```

# Memory Comparisons

![string_memory](https://github.com/user-attachments/assets/25f5acf8-9a3e-4a4c-b2f1-b2fb972cc9c8)

## Memory Usage Comparison (RSS per String)

| Crate | 0–4 chars | 0–8 chars | 0–16 chars | 0–32 chars | 0–64 chars |
| :--- | :---: | :---: | :---: | :---: | :---: |
| `std` | 36.9 B | 38.4 B | 46.8 B | 55.3 B | 71.4 B |
| `smol_str` | 24.0 B | 24.0 B | 24.0 B | 41.1 B | 72.2 B |
| `compact_str` | 24.0 B | 24.0 B | 24.0 B | 35.4 B | 61.0 B |
| `compact_string` | 24.1 B | 25.8 B | 32.6 B | 40.5 B | 56.5 B |
| **`cold-string`** | **8.0 B** | **11.2 B** | **24.9 B** | **36.5 B** | **53.5 B** |

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
