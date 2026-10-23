//! The test-support library the integration suites share, mounted into each with `#[path]`
//!
//! `drive` holds the generic codec drivers, `error` the queries on a decoder's error, `varint`
//! the LEB128 varint codecs protobuf-shaped frames are built from, `wire` the protobuf wire types
//! a tag carries and the framing each gives its value, `unknown` the codecs of the frames no field
//! declares, `frames` the frames of one proto3 message both suites build their wires from, and
//! `scans` the count of the varints a decode reads. A suite uses only part of the library, so each
//! mounts it under `#[expect(dead_code)]`
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
pub(super) mod drive;
pub(super) mod error;
pub(super) mod frames;
pub(super) mod scans;
pub(super) mod unknown;
pub(super) mod varint;
pub(super) mod wire;
