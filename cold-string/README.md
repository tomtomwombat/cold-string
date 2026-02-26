# cold-string
[![Github](https://img.shields.io/badge/github-8da0cb?style=for-the-badge&labelColor=555555&logo=github)](https://github.com/tomtomwombat/cold-string)
[![Crates.io](https://img.shields.io/badge/crates.io-fc8d62?style=for-the-badge&labelColor=555555&logo=rust)](https://crates.io/crates/cold-string)
[![docs.rs](https://img.shields.io/badge/docs.rs-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs)](https://docs.rs/cold-string)
![MSRV](https://img.shields.io/crates/msrv/cold-string?style=for-the-badge)
![Downloads](https://img.shields.io/crates/d/cold-string?style=for-the-badge)

A 1-word (8-byte) sized representation of immutable UTF-8 strings that in-lines up to 8 bytes. Optimized for memory usage and struct packing.

# Overview

`ColdString` is optimized for memory efficiency for **large** and **short** strings:
- 0..=8 bytes: always 8 bytes total (fully inlined).
- 9..=128 bytes: 8-byte pointer + 1-byte length encoding
- 129..=16384 bytes: 8-byte pointer + 2-byte length encoding
- Continues logarithmically up to 18 bytes overhead for sizes up to `isize::MAX`.

Compared to `String`, which stores capacity and length inline (3 machine words), `ColdString` avoids storing length inline for heap strings and compresses metadata into tagged pointer space. This leads to substantial memory savings in benchmarks (see [Memory Comparison (System RSS)](#memory-comparison-system-rss)):
- **36% – 68%** smaller than `String` in `HashMap`
- **28% – 65%** smaller than other short-string crates in `HashMap`
- **30% – 75%** smaller than `String` in `BTreeSet`
- **13% – 63%** smaller than other short-string crates in `BTreeSet`

`ColdString`'s MSRV is 1.60, is `no_std` compatible, and is a drop in replacement for immutable Strings.

### Safety
`ColdString` is written using [Rust's strict provenance API](https://doc.rust-lang.org/beta/std/ptr/index.html#strict-provenance), carefully handles unaligned access internally, and is validated with property testing and MIRI.

### Why "Cold"?

The heap representation stores the length on the heap, not inline in the struct. This saves memory in the struct itself but *slightly* increases the cost of `len()` since it requires a heap read. In practice, the `len()` cost is only marginally slower than inline storage and is typically negligible compared to:
- Memory savings
- Cache density improvements
- Faster collection operations due to reduced footprint

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

### Inline Mode (0 to 7 Bytes)
The tag byte has bits 11111xxx, where xxx is the length. `self.0[1]` to `self.0[7]` store the bytes of string.

### Inline Mode (8 Bytes)
The tag byte is any valid UTF-8 byte. `self.0` stores the bytes of string. Since the string is UTF-8, the tag byte is guaranteed to not be 10xxxxx or 11111xxx.

### Heap Mode
`self.0` encodes the pointer to heap, where tag byte is 10xxxxxx. 10xxxxxx is chosen because it's a UTF-8 continuation byte and therefore an impossible tag byte for inline mode. Since a heap-alignment of 4 is chosen, the pointer's least significant 2 bits are guaranteed to be 0 ([See more](https://doc.rust-lang.org/beta/std/alloc/struct.Layout.html#method.from_size_align)). These bits are swapped with the 10 "tag" bits when de/coding between `self.0` and the address value.

On the heap, the data starts with a variable length integer encoding of the length, followed by the bytes.
```text,ignore
ptr --> <var int length> <data>
```

# Memory Comparisons (Allocator)

Memory usage per string, measured by tracking the memory requested by the allocator:

![string_memory](https://github.com/user-attachments/assets/6644ae40-1da7-42e2-9ae6-0596e77e953e)

## Memory Comparison (System RSS)

RSS per insertion of various collections containing strings of random lengths 0..=N:

Vec             |   0..=4 |   0..=8 |  0..=16 |  0..=32 |  0..=64
:---              |  :---:  |  :---:  |  :---:  |  :---:  |  :---:  |
cold-string       |     8.0 |     8.0 |    23.2 |    33.7 |    53.4
compact_str       |    24.0 |    24.0 |    24.0 |    34.6 |    60.6
compact_string    |    22.9 |    24.9 |    31.6 |    39.7 |    55.7
smallstr          |    24.0 |    24.0 |    38.0 |    50.3 |    68.4
smartstring       |    24.0 |    24.0 |    24.0 |    40.4 |    65.4
smol_str          |    24.0 |    24.0 |    24.0 |    39.9 |    71.2
std               |    35.8 |    37.4 |    45.8 |    54.2 |    70.5

HashMap             |   0..=4 |   0..=8 |  0..=16 |  0..=32 |  0..=64
:---              |  :---:  |  :---:  |  :---:  |  :---:  |  :---:  |
cold-string       |    35.7 |    35.7 |    63.3 |    88.2 |   125.1
compact_str       |   102.8 |   102.8 |   102.8 |   123.7 |   175.5
compact_string    |    45.4 |    59.6 |    78.2 |    97.1 |   130.1
smallstr          |   102.8 |   102.8 |   129.7 |   155.0 |   191.6
smartstring       |   102.8 |   102.8 |   102.8 |   135.9 |   185.8
smol_str          |   102.8 |   102.8 |   102.8 |   134.8 |   196.6
std               |   112.8 |   123.9 |   143.2 |   161.8 |   195.3

B-Tree Set             |   0..=4 |   0..=8 |  0..=16 |  0..=32 |  0..=64
:---              |  :---:  |  :---:  |  :---:  |  :---:  |  :---:  |
cold-string       |    10.1 |    18.9 |    49.3 |    79.1 |   117.2
compact_str       |    24.8 |    48.4 |    61.5 |    90.5 |   145.7
compact_string    |    19.7 |    43.7 |    67.0 |    88.3 |   122.4
smallstr          |    24.8 |    48.1 |    89.7 |   121.9 |   162.0
smartstring       |    24.5 |    48.6 |    61.1 |   102.3 |   155.8
smol_str          |    25.0 |    48.3 |    61.6 |   100.7 |   166.7
std               |    35.8 |    70.4 |   102.9 |   128.9 |   165.5

**Note:** Columns represent string length (bytes/chars). Values represent average Resident Set Size (RSS) in bytes per string instance. Measurements taken with 10M iterations.

## Speed
### Construction: Variable Length (0..=N) [ns/op]
Crate              |   0..=4    |   0..=8    |   0..=16   |   0..=32   |   0..=64  
:---               |   :---:    |   :---:    |   :---:    |   :---:    |   :---:   
cold-string        |       10.0 |        9.2 |       25.3 |       30.0 |       37.2
compact_str        |        8.8 |       10.1 |       10.0 |       14.4 |       49.4
compact_string     |       34.5 |       34.8 |       37.5 |       34.9 |       38.3
smallstr           |        8.9 |        9.4 |       23.1 |       44.9 |       32.7
smartstring        |       14.8 |       15.1 |       15.0 |       26.9 |       49.5
smol_str           |       19.2 |       19.8 |       20.1 |       23.4 |       33.7
std                |       28.6 |       31.4 |       34.9 |       32.0 |       33.1

### Construction: Fixed Length (N..=N) [ns/op]
Crate              |   4..=4    |   8..=8    |  16..=16   |  32..=32   |  64..=64
:---               |   :---:    |   :---:    |   :---:    |   :---:    |   :---:
cold-string        |        6.5 |        4.2 |       34.2 |       34.3 |       36.2
compact_str        |        7.5 |        7.5 |        7.6 |       31.0 |       32.4
compact_string     |       29.2 |       28.9 |       29.2 |       29.9 |       32.3
smallstr           |        4.5 |        2.6 |       28.7 |       28.5 |       29.9
smartstring        |       14.7 |       14.8 |        8.6 |       61.6 |       63.4
smol_str           |       15.2 |       12.8 |       15.7 |       41.7 |       42.0
std                |       28.2 |       27.6 |       28.6 |       29.3 |       30.4


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
