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

impl AsRef<str> for StdArcStr {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

pub trait StringType: FromStr {
    fn name() -> &'static str;
    fn max_len() -> usize {
        usize::MAX
    }
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
    fn max_len() -> usize {
        u8::MAX as usize
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

impl StringType for cold_string::ArcColdString {
    fn name() -> &'static str {
        "cold_string::ArcColdString"
    }
}

impl StringType for cold_string::ArcColdString32 {
    fn name() -> &'static str {
        "cold_string::ArcColdString32"
    }
}

impl StringType for StdArcStr {
    fn name() -> &'static str {
        "Arc<str>"
    }
}

impl StringType for arcstr::ArcStr {
    fn name() -> &'static str {
        "arcstr::ArcStr"
    }
}

pub fn random_string<T: FromStr>(min: usize, max: usize) -> T {
    let mut rng = fastrand::Rng::with_seed(42);
    let len = rng.usize(min..=max);
    let mut scratch = [0u8; 300];
    for byte in scratch.iter_mut().take(len) {
        *byte = rng.alphanumeric() as u8;
    }
    let s = unsafe { std::str::from_utf8_unchecked(&scratch[..len]) };
    s.parse().map_err(|_| ()).unwrap()
}
