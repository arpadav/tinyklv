//! [`AttemptBudget`], which caps the decode attempts one test may make, so a decode loop that
//! never advances panics instead of hanging the suite
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::prelude::*;

// --------------------------------------------------
// external
// --------------------------------------------------
use std::{cell::Cell, fmt};
use winnow::stream::Stream;

/// A `T` whose every decode attempt counts against [`AttemptBudget::LIMIT`]
///
/// The budget is per thread: one count for the calling thread, shared by every
/// `AttemptBudget<T>` whatever its `T`, and never reset. libtest runs each test on its own
/// thread, so one test cannot spend another's budget, but every decode within a test spends from
/// the same one
#[derive(PartialEq)]
pub(crate) struct AttemptBudget<T>(T);

/// [`AttemptBudget`] implementation
impl<T> AttemptBudget<T> {
    /// Decode attempts one thread may make before its decode counts as non-terminating
    const LIMIT: u32 = 64;

    /// Counts one decode attempt on the calling thread
    ///
    /// # Panics
    ///
    /// On the attempt past [`AttemptBudget::LIMIT`]
    fn spend() {
        thread_local! {
            /// Decode attempts made on this thread
            static ATTEMPTS: Cell<u32> = const { Cell::new(0) };
        }
        let attempts = ATTEMPTS.get() + 1;
        assert!(
            attempts <= Self::LIMIT,
            "{attempts} decode attempts: the decode loop is not advancing"
        );
        ATTEMPTS.set(attempts);
    }
}

/// [`AttemptBudget`] implementation of [`fmt::Debug`]: transparent, it prints as its `T`
impl<T: fmt::Debug> fmt::Debug for AttemptBudget<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// [`AttemptBudget`] implementation of [`DecodeValue`]: one attempt, then `T`'s own decode
impl<S, T> DecodeValue<S> for AttemptBudget<T>
where
    S: Stream,
    T: DecodeValue<S>,
{
    fn decode_value(input: &mut S) -> tinyklv::Result<Self> {
        Self::spend();
        T::decode_value(input).map(Self)
    }
}
