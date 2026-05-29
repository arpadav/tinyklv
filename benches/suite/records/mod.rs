//! Shared data model for the benchmark suite.
//!
//! Each record shape every approach round-trips lives in its own file - [`Simple`] (flat
//! primitives, [`simple`]), [`Compound`] (nested sub-packet + packed run, [`compound`]), and
//! [`Rich`] (native Rust types, [`rich`]) - matching the chart tiers one-to-one. The sub-types
//! shared across records ([`Reading`], [`GpsCoord`]) live in [`shared`]. Records necessarily carry
//! each approach's `#[derive]` / attributes at the definition site, so these files are "the common
//! types plus their derives", not pure data; all procedural per-approach code lives under
//! [`super::approaches`].
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
mod compound;
mod rich;
mod shared;
mod simple;

// --------------------------------------------------
// re-exports
// --------------------------------------------------
pub(crate) use compound::Compound;
pub(crate) use rich::Rich;
pub(crate) use shared::{GpsCoord, Reading};
pub(crate) use simple::Simple;

// --------------------------------------------------
// external
// --------------------------------------------------
use rand::rngs::SmallRng;

/// Generates a random sample value for a record type, replacing standalone fixture fns
///
/// Implemented by every record type in the benchmark suite so that the criterion
/// driver and the CSV harness can each obtain varied instances. Each call advances
/// the provided RNG, producing different data on every invocation.
pub(crate) trait RngSample {
    /// Returns a random instance from the provided RNG
    ///
    /// The caller controls determinism by seeding the RNG before passing it in.
    fn rng_sample(rng: &mut SmallRng) -> Self;
}
