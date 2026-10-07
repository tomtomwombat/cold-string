# cold-string
[![Github](https://img.shields.io/badge/github-8da0cb?style=for-the-badge&labelColor=555555&logo=github)](https://github.com/tomtomwombat/cold-string)
[![Crates.io](https://img.shields.io/badge/crates.io-fc8d62?style=for-the-badge&labelColor=555555&logo=rust)](https://crates.io/crates/cold-string)
[![docs.rs](https://img.shields.io/badge/docs.rs-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs)](https://docs.rs/cold-string)
![MSRV](https://img.shields.io/crates/msrv/cold-string?style=for-the-badge)

A 1-word (8-byte) sized representation of immutable UTF-8 strings that in-lines up to 8 bytes.

## Overview

`ColdString` minimizes per-string overhead for both **short and large** strings.
- Strings ≤ 8 bytes: **8 bytes total**
- Larger strings: **~9–10 bytes overhead** (other string libraries have 24 bytes overhead)

This leads to substantial memory savings over both `String` and other short-string crates (see [Memory Comparisons](#memory-comparison-system-rss)):
- **35% – 67%** smaller `HashSet<S>`
- **30% – 75%** smaller `BTreeSet<S>` (vs `String`)
- **13% – 63%** smaller `BTreeSet<S>` (vs other crates)

---

### Portability
`ColdString`'s MSRV is 1.60, is `no_std` compatible, and is a drop in replacement for immutable Strings.

## Usage

Use it like a `String`:
```rust
use cold_string::ColdString;

let s = ColdString::new("qwerty");
assert_eq!(s.as_str(), "qwerty");
```

Use `ArcColdString` when clones should share long-string storage:

```rust
use cold_string::ArcColdString;

let first = ArcColdString::new("a string longer than one machine word");
let second = first.clone();
assert_eq!(first, second);
```

`ArcColdString` uses a native-width reference count. `ArcColdString8`,
`ArcColdString16`, and `ArcColdString32` use 1-, 2-, and 4-byte counts when a
smaller heap header is preferable. All variants remain one word in size.

Both types keep strings up to one machine word inline. For longer strings,
`ArcColdString` stores `[atomic reference count][variable-length length][UTF-8 bytes]`
in one allocation and does not support weak references.

Packs well with other types:
```rust
use cold_string::ColdString;
use std::mem::size_of;

assert_eq!(size_of::<ColdString>(), size_of::<usize>());
// ColdString has a null-niche:
assert_eq!(size_of::<Option<ColdString>>(), size_of::<ColdString>());
```

## How It Works

ColdString is an 8-byte tagged `NonNull` pointer (4 bytes on 32-bit machines) that has one of 3 representations:
- the first byte is `10xxxxxx`: The pointer encodes a heap address pointing to the length followed by the UTF-8 bytes. Rotating the `10` to the least significant position and setting to `00` decodes to the address (the heap allocation uses [4-byte alignment](https://doc.rust-lang.org/beta/std/alloc/struct.Layout.html#method.from_size_align)). The length of string is encoded with a variable length int that requires 1 bytes for 9–135 length, 2 bytes for lengths 136–16,391, etc.
- otherwise, the string is inlined: the pointer represent 0-8 UTF-8 bytes, starting from the first byte. Trailing bytes are `0xFF`, and the length is calculated from `trailing_ones`.
- the one exception to the rule above is if the whole pointer is exactly `(usize::MAX >> 4) << 2`, in which the string represents `"\0\0\0\0\0\0\0\0"` (an invalid value for `NonNull`). This number is chosen because
  - the first 2 bits are 0, classifying it as inlined,
  - it's an impossible UTF-8 representation, so doesn't collide with other inlined strings.
  - it's symmetrical, so it's the same on big or little endian.

### Why "Cold"?

The heap representation stores the length on the heap, not inline in the struct. This saves memory in the struct itself but *slightly* increases the cost of `len()` since it requires a heap read. In practice, the `len()` cost is only marginally slower than inline storage and is typically negligible compared to memory savings, cache density improvements, and 3x faster operations on inlined strings.

### Safety

`ColdString` uses `unsafe` to implement its packed representation and pointer tagging. Usage of `unsafe` is narrowly scoped to where layout control is required, and each instance is documented with `// SAFETY: <invariant>`. To further ensure soundness, `ColdString` is written using [Rust's strict provenance API](https://doc.rust-lang.org/beta/std/ptr/index.html#strict-provenance), handles unaligned access internally, maintains explicit heap alignment guarantees, and is validated with property testing and MIRI.

## Benchmarks

### Memory Comparisons (Allocator)

Memory usage per string, measured by tracking the memory requested by the allocator:

![string_memory](https://github.com/user-attachments/assets/adf09756-9910-4618-a97f-b5ab91a2515a)

### Memory Comparison (System RSS)

Resident set size in bytes per insertion of various collections. Insertions are strings with random length 0..=N:

Vec               |   0..=4 |   0..=8 |  0..=16 |  0..=32 |  0..=64
:---              |  :---:  |  :---:  |  :---:  |  :---:  |  :---:  |
cold-string       |     8.0 |     8.0 |    23.2 |    33.7 |    53.4
compact_str       |    24.0 |    24.0 |    24.0 |    34.6 |    60.6
compact_string    |    22.9 |    24.9 |    31.6 |    39.7 |    55.7
smallstr          |    24.0 |    24.0 |    38.0 |    50.3 |    68.4
smartstring       |    24.0 |    24.0 |    24.0 |    40.4 |    65.4
smol_str          |    24.0 |    24.0 |    24.0 |    39.9 |    71.2
std               |    35.8 |    37.4 |    45.8 |    54.2 |    70.5

HashSet           |   0..=4 |   0..=8 |  0..=16 |  0..=32 |  0..=64
:---              |  :---:  |  :---:  |  :---:  |  :---:  |  :---:  |
cold-string       |    18.9 |    18.9 |    34.5 |    45.5 |    64.0
compact_str       |    52.4 |    52.4 |    52.4 |    62.2 |    88.9
compact_string    |    23.2 |    30.0 |    39.6 |    49.1 |    65.9
smallstr          |    52.4 |    52.4 |    66.5 |    78.6 |    96.9
smartstring       |    52.4 |    52.4 |    52.4 |    68.2 |    94.0
smol_str          |    52.4 |    52.4 |    52.4 |    68.3 |    99.4
std               |    56.8 |    61.9 |    72.2 |    81.7 |    98.5

BTreeSet          |   0..=4 |   0..=8 |  0..=16 |  0..=32 |  0..=64
:---              |  :---:  |  :---:  |  :---:  |  :---:  |  :---:  |
cold-string       |    10.1 |    18.9 |    49.3 |    79.1 |   117.2
compact_str       |    24.8 |    48.4 |    61.5 |    90.5 |   145.7
compact_string    |    19.7 |    43.7 |    67.0 |    88.3 |   122.4
smallstr          |    24.8 |    48.1 |    89.7 |   121.9 |   162.0
smartstring       |    24.5 |    48.6 |    61.1 |   102.3 |   155.8
smol_str          |    25.0 |    48.3 |    61.6 |   100.7 |   166.7
std               |    35.8 |    70.4 |   102.9 |   128.9 |   165.5

### Speed
#### Construction: Variable Length (0..=N) [ns/op]
Crate              |   0..=4    |   0..=8    |   0..=16   |   0..=32   |   0..=64  
:---               |   :---:    |   :---:    |   :---:    |   :---:    |   :---:   
cold-string        |       10.0 |        9.2 |       25.3 |       30.0 |       37.2
compact_str        |        8.8 |       10.1 |       10.0 |       14.4 |       49.4
compact_string     |       34.5 |       34.8 |       37.5 |       34.9 |       38.3
smallstr           |        8.9 |        9.4 |       23.1 |       44.9 |       32.7
smartstring        |       14.8 |       15.1 |       15.0 |       26.9 |       49.5
smol_str           |       19.2 |       19.8 |       20.1 |       23.4 |       33.7
std                |       28.6 |       31.4 |       34.9 |       32.0 |       33.1

#### Construction: Fixed Length (N..=N) [ns/op]
Crate              |   4..=4    |   8..=8    |  16..=16   |  32..=32   |  64..=64
:---               |   :---:    |   :---:    |   :---:    |   :---:    |   :---:
cold-string        |        6.5 |        4.2 |       34.2 |       34.3 |       36.2
compact_str        |        7.5 |        7.5 |        7.6 |       31.0 |       32.4
compact_string     |       29.2 |       28.9 |       29.2 |       29.9 |       32.3
smallstr           |        4.5 |        2.6 |       28.7 |       28.5 |       29.9
smartstring        |       14.7 |       14.8 |        8.6 |       61.6 |       63.4
smol_str           |       15.2 |       12.8 |       15.7 |       41.7 |       42.0
std                |       28.2 |       27.6 |       28.6 |       29.3 |       30.4

### Reference Counting Memory Comparison

Theoretical overhead on a 64-bit target, excluding one pointer and the UTF-8
payload. Inline values have no pointer or heap allocation, so their overhead is
shown relative to the string bytes alone. Allocator rounding is not included.

Type              | 8 bytes    | 128 bytes | 512 bytes
:---              |       ---: |       ---: |       ---:
`Arc<str>`        |         24 |         24 |         24
`arcstr::ArcStr`  |         16 |         16 |         16
`ArcColdString`   | 0 (inline) |          9 |         10
`ArcColdString32` | 0 (inline) |          5 |          6

RSS bytes per unique string in a pre-sized `Vec`, measured on 64-bit Windows
with 1,000,000 strings per subprocess. Each cell is the median of three isolated
release-mode runs and includes the `Vec` element, heap allocation, and allocator
rounding. RSS results will vary by operating system and allocator.

Type                              |   4..=4 |   8..=8 | 16..=16 | 32..=32 | 64..=64
:---                              |  :---:  |  :---:  |  :---:  |  :---:  |  :---:  |
`cold_string::ArcColdString`      |     8.0 |     8.0 |    56.3 |    70.2 |   102.4
`cold_string::ArcColdString32`    |     8.0 |     8.0 |    38.2 |    54.5 |    86.3
`arcstr::ArcStr`                  |    38.3 |    38.4 |    54.4 |    70.5 |   102.5
`Arc<str>`                        |    46.3 |    46.4 |    62.4 |    78.5 |   110.6

Type                              |   0..=4 |   0..=8 |  0..=16 |  0..=32 |  0..=64
:---                              |  :---:  |  :---:  |  :---:  |  :---:  |  :---:  |
`cold_string::ArcColdString`      |     8.0 |     8.0 |    23.2 |    38.0 |    58.0
`cold_string::ArcColdString32`    |     8.0 |     8.0 |    21.7 |    35.9 |    55.5
`arcstr::ArcStr`                  |    32.0 |    34.8 |    44.1 |    53.0 |    69.6
`Arc<str>`                        |    46.4 |    46.4 |    54.0 |    62.1 |    77.9


### Reference Counting Speed Comparison

Criterion point estimates in nanoseconds per operation on 64-bit Windows
(release mode, 100 samples). Lower is better. Allocation and clone outputs are
dropped outside the timed region; drop measures a non-final reference-count
decrement.

Fresh allocation and string copy:

Type             | 16 bytes | 128 bytes | 512 bytes
:---             |     ---: |      ---: |      ---:
`Arc<str>`       |     28.03 |      29.57 |      36.98
`arcstr::ArcStr` |     27.19 |      28.70 |      38.46
`ArcColdString`  |     30.12 |      32.61 |      43.70

Clone/reference-count increment:

Type             | 16 bytes | 128 bytes | 512 bytes
:---             |     ---: |      ---: |      ---:
`Arc<str>`       |      3.86 |       3.82 |       3.82
`arcstr::ArcStr` |      3.39 |       3.41 |       3.43
`ArcColdString`  |      3.44 |       3.45 |       3.43

String access:

Type             | 16 bytes | 128 bytes | 512 bytes
:---             |     ---: |      ---: |      ---:
`Arc<str>`       |      1.02 |       1.03 |       1.04
`arcstr::ArcStr` |      1.02 |       1.02 |       1.02
`ArcColdString`  |      1.38 |       1.68 |       1.70

Non-final drop/reference-count decrement:

Type             | 16 bytes | 128 bytes | 512 bytes
:---             |     ---: |      ---: |      ---:
`Arc<str>`       |      2.43 |       2.49 |       2.43
`arcstr::ArcStr` |      2.52 |       2.53 |       2.52
`ArcColdString`  |      2.17 |       2.18 |       2.19

Clone and drop contention across 4 threads sharing strings in a fixed size pool:
Type                          |  Light (1024)  |  Medium (16)   |   Heavy (1)   
:---                          |     :---:      |     :---:      |     :---:     
`Arc<str>`                    |          26.55 |          54.69 |         144.81
`arcstr::ArcStr`              |          29.29 |         119.39 |         214.31
`cold_string::ArcColdString32`|           25.69 |          90.53 |         262.83

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
