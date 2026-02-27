#![cfg_attr(docsrs, doc(cfg(feature = "rkyv")))]

use crate::ColdString;

use rkyv::{
    rancor::{Fallible, Source},
    ser::{Allocator, Writer},
    string::{ArchivedString, StringResolver},
    Archive, Deserialize, Place, Serialize,
};

impl Archive for ColdString {
    type Archived = ArchivedString;
    type Resolver = StringResolver;

    #[inline]
    fn resolve(&self, resolver: Self::Resolver, out: Place<Self::Archived>) {
        ArchivedString::resolve_from_str(self, resolver, out);
    }
}

impl<S> Serialize<S> for ColdString
where
    S: Fallible + Allocator + Writer + ?Sized,
    S::Error: Source,
{
    #[inline]
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        ArchivedString::serialize_from_str(self, serializer)
    }
}

impl<D: Fallible + ?Sized> Deserialize<ColdString, D> for ArchivedString {
    #[inline]
    fn deserialize(&self, _deserializer: &mut D) -> Result<ColdString, D::Error> {
        Ok(ColdString::new(self.as_str()))
    }
}

impl PartialEq<ColdString> for ArchivedString {
    #[inline]
    fn eq(&self, other: &ColdString) -> bool {
        other.as_str() == self.as_str()
    }
}

impl PartialEq<ArchivedString> for ColdString {
    #[inline]
    fn eq(&self, other: &ArchivedString) -> bool {
        other.as_str() == self.as_str()
    }
}

impl PartialOrd<ColdString> for ArchivedString {
    #[inline]
    fn partial_cmp(&self, other: &ColdString) -> Option<::core::cmp::Ordering> {
        Some(self.as_str().cmp(other.as_str()))
    }
}

impl PartialOrd<ArchivedString> for ColdString {
    #[inline]
    fn partial_cmp(&self, other: &ArchivedString) -> Option<::core::cmp::Ordering> {
        Some(self.as_str().cmp(other.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rkyv::rancor::Error;

    #[cfg_attr(miri, ignore)] // https://github.com/rust-lang/unsafe-code-guidelines/issues/134
    #[test]
    fn roundtrip_cold_string() {
        for s in ["", "hello", "this is a longer cold string"] {
            let data = ColdString::from(s);
            let bytes = rkyv::to_bytes::<Error>(&data).unwrap();
            let archived =
                rkyv::access::<rkyv::Archived<ColdString>, rkyv::rancor::Error>(&bytes).unwrap();
            assert_eq!(&data, archived);
            let deserialized: ColdString =
                rkyv::deserialize::<ColdString, Error>(archived).unwrap();
            assert_eq!(data, deserialized);

            let bytes = rkyv::to_bytes::<Error>(&data).unwrap();
            let deserialized = rkyv::from_bytes::<ColdString, Error>(&bytes).unwrap();
            assert_eq!(data, deserialized);
        }
    }
}
