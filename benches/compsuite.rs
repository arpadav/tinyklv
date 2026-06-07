#![allow(clippy::indexing_slicing)]
//! tinyklv benchmark suite: the four KLV approaches (tinyklv, serde_klv, tlv_parser,
//! hand-written manual) against four protobuf crates (prost, quick_protobuf, rust_protobuf,
//! micropb) round-tripping the same records.
//!
//! One criterion binary drives the whole comparison. The `suite` module tree holds the
//! shared data model ([`suite::records`]), framing primitives ([`suite::framing`]), the
//! cross-approach equivalence gate ([`suite::checks`]), and one directory per approach
//! under [`suite::approaches`]. Every approach implements [`suite::Codec`] for all three record
//! shapes, so the per-approach `simple.rs` / `compound.rs` / `rich.rs` files line up
//! method-for-method and can be diffed side by side - tinyklv's are tiny next to the
//! hand-written and protobuf-wrapper ones, which is the maintainability story the suite
//! exists to tell.
//!
//! Author: aav
extern crate alloc; // needed for micropb

// --------------------------------------------------
// mods
// --------------------------------------------------
#[path = "suite/mod.rs"]
mod suite;

// --------------------------------------------------
// local
// --------------------------------------------------
use suite::records::{Compound, Rich, Simple};

// --------------------------------------------------
// external
// --------------------------------------------------
use criterion::{Criterion, criterion_group, criterion_main};

/// Runs every group for all record shapes: 3 records x 5 tiers x 8 approaches.
///
/// Record shapes run simplest to richest: `simple` (flat primitives), `compound` (nested
/// sub-record + packed run), `rich` (native domain types - dates, addresses, non-zero ints).
fn all_benches(c: &mut Criterion) {
    write_suite_meta();
    suite::run::<Simple>(c, "simple");
    suite::run::<Compound>(c, "compound");
    suite::run::<Rich>(c, "rich");
}

/// Writes a tiny sidecar beside criterion's output recording the per-tier element count
///
/// Criterion's own [`criterion::Throughput`] already carries the per-bar byte length (used by the
/// chart's `byte` normalization), and it can hold only one metric per benchmark. The `streamed`
/// tier decodes [`suite::framing::STREAM_PACKETS`] packets per call, which the chart's `pkt`
/// normalization divides by; persisting that count here keeps it single-sourced in Rust rather than
/// duplicated in the plotting script.
fn write_suite_meta() {
    // --------------------------------------------------
    // one number the renderer needs and criterion cannot co-store
    // --------------------------------------------------
    let meta = format!(
        "{{\"stream_packets\":{}}}\n",
        suite::framing::STREAM_PACKETS
    );
    let dir = std::path::Path::new("target/criterion");
    let _ = std::fs::create_dir_all(dir);
    let _ = std::fs::write(dir.join("suite_meta.json"), meta);
}

criterion_group!(suite, all_benches);
criterion_main!(suite);
