//! `#[derive(Klv)]` inputs that must not compile, each pinned by message and location
//!
//! Every `fail/*.rs` case has a committed `.stderr` snapshot of its diagnostic. Regenerate the
//! snapshots with `TRYBUILD=overwrite cargo +1.95.0 test --test ui`
//!
//! Author: aav

/// Every case under `fail/` fails to compile with exactly its `.stderr`
#[test]
fn ui() {
    trybuild::TestCases::new().compile_fail("tests/ui/fail/*.rs");
}
