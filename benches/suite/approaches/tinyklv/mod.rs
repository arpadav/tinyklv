//! The tinyklv approach: one `#[derive(Klv)]` on the record (in
//! [`records`](crate::suite::records)) and a one-line trait call per operation. No adapter
//! module - that absence is the point. Compare the size of these files to the sibling
//! approaches.
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
mod simple;
mod rich;
mod compound;

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::suite::Approach;

/// The tinyklv approach.
pub(crate) struct Tinyklv;

impl Approach for Tinyklv {
    const NAME: &'static str = "tinyklv";
}
