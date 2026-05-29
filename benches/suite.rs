//! tinyklv benchmark suite: the four KLV approaches (tinyklv, serde_klv, tlv_parser,
//! hand-written manual) against four protobuf crates (prost, quick_protobuf, rust_protobuf,
//! micropb) round-tripping the same records.
//!
//! One criterion binary drives the whole comparison. The `suite` module tree holds the
//! shared data model ([`suite::records`]), framing primitives ([`suite::framing`]), the
//! cross-approach equivalence gate ([`suite::checks`]), and one directory per approach
//! under [`suite::approaches`]. Every approach implements [`suite::Codec`] for both record
//! shapes, so the eight `<approach>/flat.rs` (and `<approach>/nested.rs`) files line up
//! method-for-method and can be diffed side by side - tinyklv's are tiny next to the
//! hand-written and protobuf-wrapper ones, which is the maintainability story the suite
//! exists to tell.
//!
//! Author: aav
// the micropb approach's generated code references `::alloc`; bring it into the crate root
extern crate alloc;

#[path = "suite/mod.rs"]
mod suite;

// --------------------------------------------------
// local
// --------------------------------------------------
use suite::records::{NativeNested, Platform, Telemetry};

// --------------------------------------------------
// external
// --------------------------------------------------
use criterion::{criterion_group, criterion_main, Criterion};

/// Runs every group for all record shapes: 3 records x 4 operations x 8 approaches = 96.
fn all_benches(c: &mut Criterion) {
    suite::run::<Telemetry>(c, "flat");
    suite::run::<Platform>(c, "nested");
    suite::run::<NativeNested>(c, "native_nested");
}

criterion_group!(suite, all_benches);
criterion_main!(suite);
