/// Seeks to the beginning of the prescribed type from a stream using the sentinel
///
/// Encode counterpart: sentinel bytes are prepended by [`EncodeFrame`](crate::traits::EncodeFrame)
///
/// This is automatically implemented when `sentinel` is set in the [`crate::Klv`](crate::Klv) attribute
pub trait SeekSentinel<S>: Sized
where
    S: winnow::stream::Stream,
{
    fn seek_sentinel(input: &mut S) -> crate::Result<S>;
}
