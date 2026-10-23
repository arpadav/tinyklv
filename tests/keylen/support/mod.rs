//! Support modules shared by the fused-`keylen` test files
//!
//! `header` holds hand-written frame header codecs, protobuf's and a bare `u8`-key header;
//! `value` the hand-written value codecs of a string and a packed repeated; `example` the
//! canonical protobuf message and the small `Reading` most tests share; and `header_budget` the
//! cap on header reads that turns a hang into a panic
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
pub(super) mod example;
pub(super) mod header;
pub(super) mod header_budget;
pub(super) mod value;
