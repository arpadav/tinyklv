//! [`HeaderBudget`], which caps the frame headers one test may read, so a fused decode loop that
//! never advances panics instead of hanging the suite
//!
//! Author: aav
// --------------------------------------------------
// external
// --------------------------------------------------
use std::cell::Cell;

/// The frame headers the calling thread may read before its decode loop counts as
/// non-terminating
///
/// A test's `keylen` decoder spends one read per call, and a loop that never advances calls it
/// without end, so the budget bounds the loop in reads rather than in time. The budget is per
/// thread and never reset: libtest runs each test on its own thread, so one test cannot spend
/// another's budget, but every decode within a test spends from the same one
pub(crate) struct HeaderBudget;

/// [`HeaderBudget`] implementation
impl HeaderBudget {
    /// Headers one thread may read
    const LIMIT: u32 = 1024;

    /// Counts one header read on the calling thread
    ///
    /// # Panics
    ///
    /// On the read past [`HeaderBudget::LIMIT`]
    pub(crate) fn spend() {
        thread_local! {
            /// Frame headers read on this thread
            static READS: Cell<u32> = const { Cell::new(0) };
        }
        let reads = READS.get() + 1;
        assert!(
            reads <= Self::LIMIT,
            "{reads} header reads: the decode loop is not advancing"
        );
        READS.set(reads);
    }
}
