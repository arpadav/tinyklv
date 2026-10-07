//! Support modules shared by the fused-`keylen` test files
//!
//! `pb` holds hand-written wire codecs, protobuf's and a bare `u8`-key header; `example` the
//! canonical protobuf message and the small `Reading` most tests share; and `header_budget` the
//! cap on header reads that turns a hang into a panic
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
pub(super) mod example;
pub(super) mod header_budget;
pub(super) mod pb;
