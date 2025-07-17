// --------------------------------------------------
// external
// --------------------------------------------------
use winnow::error::ContextError;

// --------------------------------------------------
// local
// --------------------------------------------------
pub use crate::prelude::*;

/// Trait for decoding from stream-type T, of type [`winnow::stream::Stream`]
/// 
/// Common examples of stream types include `&[u8]` and `&str`
/// 
/// Automatically implemented for structs deriving the [`tinyklv::Klv`](crate::Klv) trait which have decoders for every field covered.
/// 
/// For custom decoding functions, ***no need to use this trait***. Instead, please ensure the functions signature matches the following:
/// 
/// * fixed length:     `fn <name>(input: &mut S)   -> tinyklv::Result<Self>;`
/// * variable length:  `fn <name>(len: usize)      -> impl Fn(&mut S) -> tinyklv::Result<Self>;`
pub trait Decode<S>: Sized
where
    S: winnow::stream::Stream,
{
    fn decode(input: &mut S) -> winnow::Result<Self>;
}

/// Trait for seeking to the beginning of the prescribed type from a stream
/// 
/// This is automatically implemented when `sentinel` is set in the [`crate::Klv`](crate::Klv) attribute
pub trait Seek<S>: Sized
where
    S: winnow::stream::Stream,
{
    fn seek(input: &mut S) -> winnow::Result<S>;
}

/// Trait for extracting from stream-type `T`, of type [`winnow::stream::Stream`]
pub trait Extract<S>: Sized
where
    S: winnow::stream::Stream,
{
    fn extract(input: &mut S) -> winnow::Result<Self>;
}
/// [`Extract`] implementation for all types `T` that implement [`Seek`] and [`Decode`]
impl<S, T> Extract<S> for T
where
    S: winnow::stream::Stream,
    T: Seek<S> + Decode<S>,
{
    fn extract(input: &mut S) -> winnow::Result<Self> {
        let mut sought = T::seek.parse_next(input)?;
        let result = T::then_decode(&mut sought).parse_next(input);
        result
    }
}

/// Internal trait for parsing and decoding embedded data
/// 
/// See [`Extract`] for more information
/// 
/// Idea is: 
/// 
/// * [`Seek`] finds the data using the recognition sentinel
/// * [`Decode`] decodes the data of the packet, without finding it
/// * [`Extract`] performs [`Seek`] -> [`Decode`]. But upon failure, it has to return the checkpoint to the next item of input, rather than the checkpoint of the sub-slice used in the [`Decode`] call
/// 
/// [`ThenDecode`] solves this issue by taking the sub-slice as an input, passing it to the [`Decode`] implementation, and upon failure, returning to the original input checkpoint.
trait ThenDecode<S>: Sized
where
    S: winnow::stream::Stream,
{
    fn then_decode(subslice: &mut S) -> impl FnMut(&mut S) -> winnow::Result<Self>;
}
/// [`ThenDecode`] implementation for all types `T` that implement [`Decode`]
impl<S, T> ThenDecode<S> for T
where
    S: winnow::stream::Stream,
    T: Decode<S>,
{
    fn then_decode(subslice: &mut S) -> impl FnMut(&mut S) -> winnow::Result<Self> {
        move |input: &mut S| {
            let checkpoint = input.checkpoint();
            match Self::decode(subslice) {
                Ok(parsed) => Ok(parsed),
                Err(e) => Err(e.add_context(
                    input,
                    &checkpoint,
                    winnow::error::StrContext::Label("Unable to parse data embedded in packet"),
                
                )),
            }
        }
    }
}

/// Decodes repeatedly, until it can no longer
/// 
/// Accumulates results in a [`Vec`] and returns
/// 
/// Note that this **always** returns [`Ok`]: if there is a failure, it will return [`Ok`] with [an empty vector](Vec::new)
pub trait RepeatedDecode<S>: Sized
where
    S: winnow::stream::Stream,
{
    fn repeated(input: &mut S) -> winnow::Result<Vec<Self>>;
}
/// [`RepeatedDecode`] implementation for all types `T` that implement [`Decode`]
impl<S, T> RepeatedDecode<S> for T
where
    T: Decode<S>,
    S: winnow::stream::Stream,
{
    fn repeated(input: &mut S) -> winnow::Result<Vec<Self>> {
        Ok(winnow::combinator::repeat(0.., Self::decode)
            .parse_next(input)
            .unwrap_or_default()
        )
    }
}

/// Decoding-loop break types
pub enum BreakConditionType {
    /// Do nothing in the decoding loop in [`crate::prelude::Decode::decode`].
    /// 
    /// This is the default, it just means there is nothing to be done and 
    /// continue the decoding loop. 
    /// 
    /// Is named [`BreakConditionType::Proceed`] to refrain from using the keyword
    /// `continue`, since this does not use the reserved word `continue` and
    /// skip anything in the loop. 
    /// 
    /// This is equivalent to:
    /// 
    /// ```rust ignore
    /// loop {
    ///     match Self::break_condition(key, len) {
    ///         BreakConditionType::Proceed => (),
    ///         _ => ..., // see `BreakConditionType`
    ///     }
    ///     // decoding logic
    /// }
    /// // return
    /// ```
    Proceed,
    
    /// Skips the current value in [`crate::prelude::Decode::decode`]
    /// 
    /// This is useful for skipping fields that aren't implemented yet,
    /// by skipping over them and continuing the decoding loop.
    /// 
    /// This is equivalent to:
    /// 
    /// ```rust ignore
    /// loop {
    ///     match Self::break_condition(key, len) {
    ///         BreakConditionType::Skip => {
    ///             take(len).parse_next(input); // error handled properly in macro expansion
    ///             continue;
    ///         },
    ///         _ => ..., // see `BreakConditionType`
    ///     }
    ///     // decoding logic
    /// }
    /// // return
    /// ```
    Skip,

    /// Returns the decoded value, if all required fields are present.
    /// 
    /// This does not guarantee to return [`Ok`] from
    /// [`crate::prelude::Decode::decode`], since required fields might not
    /// be present.
    /// 
    /// This is useful if some un-recoverable issue has occurred but we
    /// still want to return a partially-parsed result.
    /// 
    /// For example, if `len` is decoded to be greater than the maximum
    /// size of the packet, then clearly the packet is malformed and we
    /// should at least try to return what has already been parsed.
    /// 
    /// This is equivalent to:
    /// 
    /// ```rust ignore
    /// loop {
    ///     match Self::break_condition(key, len) {
    ///         BreakConditionType::Done => break,
    ///         _ => ..., // see `BreakConditionType`
    ///     }
    ///     // decoding logic
    /// }
    /// return Ok(/* check if required fields are present */);
    /// ```
    Done,
    
    /// Returns an error from [`crate::prelude::Decode::decode`]
    /// without returning any potential decoded values.
    /// 
    /// This is should only be used in cases of a fatal error, since an
    /// [`Err`] is **guaranteed** to return from [`crate::prelude::Decode::decode`].
    /// 
    /// This is equivalent to:
    /// 
    /// ```rust ignore
    /// loop {
    ///     match Self::break_condition(key, len) {
    ///         BreakConditionType::Abort(e) => return Err(e),
    ///         _ => ..., // see `BreakConditionType`
    ///     }
    ///     // decoding logic
    /// }
    /// // return
    /// ```
    Abort(ContextError),
}

/// A trait for breaking during during decoding loop
pub trait BreakCondition<S> {
    #[inline(always)]
    #[allow(unused_variables)]
    fn break_condition<K, L>(decoded_key: K, decoded_len: L) -> BreakConditionType {
        BreakConditionType::Proceed
    }
}
/// [`BreakCondition`] implementation of [`Decode`] for all types `T` that implement [`Decode`]
impl<T, S> BreakCondition<S> for T
where
    T: crate::traits::dec::Decode<S>,
    S: winnow::stream::Stream,
{}