//! Unit tests for domain/props.rs.

use super::*;

#[test]
fn levels_match_within_a_point() {
    assert!(same_level(100.0, 100.4));
    assert!(!same_level(0.0, 120.0));
}
