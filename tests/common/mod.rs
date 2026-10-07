//! The test-support library the integration suites share, mounted into each with `#[path]`
//!
//! `drive` holds the generic codec drivers, `error` the queries on a decoder's error, and `varint`
//! the LEB128 varint codecs protobuf-shaped frames are built from. A suite uses only part of the
//! library, so each mounts it under `#[expect(dead_code)]`
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
pub(super) mod drive;
pub(super) mod error;
pub(super) mod varint;
