use std::str::FromStr;

pub fn random_string<T: FromStr>(min: usize, max: usize) -> T {
    let len = fastrand::usize(min..=max);
    let mut scratch = [0u8; 128];
    for i in 0..len {
        scratch[i] = fastrand::alphanumeric() as u8;
    }
    let s = unsafe { std::str::from_utf8_unchecked(&scratch[..len]) };
    s.parse().map_err(|_| ()).unwrap()
}
