#![deny(unused_imports)]

use std::str::FromStr;
use std::sync::Arc;

#[derive(Clone, Hash, Eq, PartialEq)]
#[repr(transparent)]
pub struct StdArcStr(Arc<str>);

impl FromStr for StdArcStr {
    type Err = std::convert::Infallible;
    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Arc::<str>::from(s)))
    }
}

pub trait StringType {
    fn name() -> &'static str;
}

impl StringType for String {
    fn name() -> &'static str {
        "String"
    }
}

impl StringType for cold_string::ColdString {
    fn name() -> &'static str {
        "cold_string::ColdString"
    }
}

impl StringType for compact_string::CompactString {
    fn name() -> &'static str {
        "compact_string::CompactString"
    }
}

impl StringType for compact_str::CompactString {
    fn name() -> &'static str {
        "compact_str::CompactString"
    }
}

impl StringType for smartstring::alias::String {
    fn name() -> &'static str {
        "smartstring::alias::String"
    }
}

impl StringType for smallstr::SmallString<[u8; 8]> {
    fn name() -> &'static str {
        "smallstr::SmallString<[u8; 8]>"
    }
}

impl StringType for smol_str::SmolStr {
    fn name() -> &'static str {
        "smol_str::SmolStr"
    }
}

pub fn random_string<T: FromStr>(min: usize, max: usize) -> T {
    let len = fastrand::usize(min..=max);
    let mut scratch = [0u8; 255];
    for byte in scratch.iter_mut().take(len) {
        *byte = fastrand::alphanumeric() as u8;
    }
    let s = unsafe { std::str::from_utf8_unchecked(&scratch[..len]) };
    s.parse().map_err(|_| ()).unwrap()
}
