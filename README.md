# cold-string
[![Github](https://img.shields.io/badge/github-8da0cb?style=for-the-badge&labelColor=555555&logo=github)](https://github.com/tomtomwombat/cold-string)
[![Crates.io](https://img.shields.io/badge/crates.io-fc8d62?style=for-the-badge&labelColor=555555&logo=rust)](https://crates.io/crates/cold-string)
[![docs.rs](https://img.shields.io/badge/docs.rs-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs)](https://docs.rs/cold-string)
![Downloads](https://img.shields.io/crates/d/cold-string?style=for-the-badge)

Compact string optimized for memory usage and struct packing.

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

# Memory Comparisons

TODO

## Measured from System Memory

### 0..=4
```ignore
Crate, len 0..=4   |      RSS (B) |  Virtual (B)
-------------------|--------------|-------------
std                |         36.9 |         38.4
smol_str           |         24.0 |         24.0
compact_str        |         24.0 |         24.0
compact_string     |         24.1 |         26.2
cold-string        |          8.0 |          8.0
```

### 0..=8
```ignore
Crate, len 0..=8   |      RSS (B) |  Virtual (B)
-------------------|--------------|-------------
std                |         38.4 |         40.0
smol_str           |         24.0 |         24.0
compact_str        |         24.0 |         24.0
compact_string     |         25.8 |         27.8
cold-string        |         11.2 |         11.7
```

### 0..=16
```ignore
Crate, len 0..=16  |      RSS (B) |  Virtual (B)
-------------------|--------------|-------------
std                |         46.8 |         48.6
smol_str           |         24.0 |         24.1
compact_str        |         24.0 |         24.0
compact_string     |         32.6 |         34.9
cold-string        |         24.9 |         26.7
```

### 0..=32
```ignore
Crate, len 0..=32  |      RSS (B) |  Virtual (B)
-------------------|--------------|-------------
std                |         55.3 |         57.4
smol_str           |         41.1 |         42.1
compact_str        |         35.4 |         36.6
compact_string     |         40.5 |         42.9
cold-string        |         36.5 |         38.8
```

### 0..=64
```ignore
Crate, len 0..=64  |      RSS (B) |  Virtual (B)
-------------------|--------------|-------------
std                |         71.4 |         73.7
smol_str           |         72.2 |         74.3
compact_str        |         61.0 |         63.3
compact_string     |         56.5 |         59.1
cold-string        |         53.5 |         56.3
```


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