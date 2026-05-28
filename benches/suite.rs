//! tinyklv benchmark suite: tinyklv vs serde_klv vs tlv_parser vs hand-written manual.
//!
//! One criterion binary drives the whole comparison. The `suite` module tree holds the
//! shared data model ([`suite::records`]), framing primitives ([`suite::framing`]), the
//! cross-approach equivalence gate ([`suite::checks`]), and one directory per approach
//! under [`suite::approaches`]. Every approach implements [`suite::Codec`] for both record
//! shapes, so the four `<approach>/flat.rs` (and `<approach>/nested.rs`) files line up
//! method-for-method and can be diffed side by side - tinyklv's are tiny next to the
//! hand-written ones, which is the maintainability story the suite exists to tell.
//!
//! Author: aav
#[path = "suite/mod.rs"]
mod suite;

// --------------------------------------------------
// local
// --------------------------------------------------
use suite::records::{Platform, Telemetry};

// --------------------------------------------------
// external
// --------------------------------------------------
use criterion::{criterion_group, criterion_main, Criterion};

/// Runs every group for both record shapes: 2 records x 4 operations x 4 approaches = 32.
fn all_benches(c: &mut Criterion) {
    suite::run::<Telemetry>(c, "flat");
    suite::run::<Platform>(c, "nested");
}

criterion_group!(suite, all_benches);
criterion_main!(suite);
