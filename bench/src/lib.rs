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

pub fn random_string<T: FromStr>(min: usize, max: usize) -> T {
    let len = fastrand::usize(min..=max);
    let mut scratch = [0u8; 128];
    for byte in scratch.iter_mut().take(len) {
        *byte = fastrand::alphanumeric() as u8;
    }
    let s = unsafe { std::str::from_utf8_unchecked(&scratch[..len]) };
    s.parse().map_err(|_| ()).unwrap()
}
