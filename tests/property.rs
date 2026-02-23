use cold_string::*;
use proptest::prelude::*;

#[cfg(miri)]
fn proptest_config() -> ProptestConfig {
    ProptestConfig {
        failure_persistence: None,
        cases: 4,
        ..Default::default()
    }
}

#[cfg(not(miri))]
fn proptest_config() -> ProptestConfig {
    ProptestConfig::with_cases(65536)
}

proptest! {
    #![proptest_config(proptest_config())]

    #[test]
    fn arb_string_eq((left, right) in any::<(String, String)>()) {
        let cold1 = ColdString::new(left.as_str());
        let cold2 = ColdString::new(right.as_str());
        assert_eq!(cold1 == cold2, left == right);
        assert_eq!(cold1 == right.as_str(), left == right);
        assert_eq!(right.as_str() == cold1, left == right);
        assert_eq!(cold2 == left.as_str(), left == right);
        assert_eq!(left.as_str() == cold2, left == right);
    }

    #[test]
    fn arb_string(s in any::<String>()) {
        let cold = ColdString::new(s.as_str());
        assert_eq!(s.len() <= 8, cold.is_inline());
        assert_eq!(cold.len(), s.len());
        assert_eq!(cold.as_str(), s.as_str());
        assert_eq!(cold, ColdString::from(s.as_str()));
        assert_eq!(cold, cold.clone());
    }

}
