//! Shared assertions for fixture identity integrity, independent of serialization formats.
use std::collections::HashSet;

/// Rejects empty suites, unnamed cases and duplicate identities before execution.
/// The owning test module remains responsible for loading and validating its data.
#[track_caller]
pub fn assert_unique_ids<'a>(ids: impl IntoIterator<Item = &'a str>) {
    let mut seen = HashSet::new();
    for id in ids {
        assert!(!id.is_empty(), "fixture identities must not be empty");
        assert!(seen.insert(id), "duplicate fixture identity: {id}");
    }
    assert!(!seen.is_empty(), "fixture suite must not be empty");
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn independent_suites_may_reuse_case_names() {
        assert_unique_ids(["first", "second"]);
        assert_unique_ids(["first"]);
    }
    #[test]
    fn empty_unnamed_and_duplicate_cases_are_rejected() {
        for invalid in [vec![], vec![""], vec!["same", "same"]] {
            assert!(std::panic::catch_unwind(|| assert_unique_ids(invalid)).is_err());
        }
    }
}
