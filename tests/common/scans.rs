//! [`Scans`], which counts the varints the calling thread decodes, so a test can tell how many
//! times a decode loop read each value
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::varint;
use tinyklv::Result;

// --------------------------------------------------
// external
// --------------------------------------------------
use std::cell::Cell;

// --------------------------------------------------
// statics
// --------------------------------------------------
thread_local! {
    /// Calls to [`Scans::dec_u64`] on this thread since [`Scans::count`] last reset it
    static SCANS: Cell<u32> = const { Cell::new(0) };
}

/// The varints the calling thread decodes through [`Scans::dec_u64`]
pub(crate) struct Scans;

/// [`Scans`] implementation
impl Scans {
    /// Decodes one varint as a `u64`, counting the call
    ///
    /// # Arguments
    ///
    /// * `input` - the byte stream to read from; advanced, or left on a failure, as
    ///   [`varint::dec_u64`] does
    ///
    /// # Returns
    ///
    /// The result of [`varint::dec_u64`]
    pub(crate) fn dec_u64(input: &mut &[u8]) -> Result<u64> {
        SCANS.set(SCANS.get() + 1);
        varint::dec_u64(input)
    }

    /// Runs `decode` from a count of zero
    ///
    /// # Arguments
    ///
    /// * `decode` - the decode whose varint reads are counted
    ///
    /// # Returns
    ///
    /// What `decode` returns, and the calls to [`Scans::dec_u64`] it made on the calling thread
    pub(crate) fn count<R>(decode: impl FnOnce() -> R) -> (R, u32) {
        SCANS.set(0);
        let decoded = decode();
        (decoded, SCANS.get())
    }
}
