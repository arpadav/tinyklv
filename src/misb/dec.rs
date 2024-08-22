// --------------------------------------------------
// external
// --------------------------------------------------
use chrono::TimeZone;
use winnow::prelude::*;
use winnow::stream::Stream;
use winnow::error::AddContext;

/// Represents the number of MICROSECONDS elapsed since midnight
/// (00:00:00), January 1, 1970, not including leap seconds.
/// 
/// # Example
/// 
/// ```
/// use chrono::TimeZone;
/// use tinyklv::prelude::*;
/// use tinyklv::misb::dec::precision_timestamp;
/// let mut val1: &[u8] = &(0x0004_59F4_A6AA_4AA8 as u64).to_be_bytes();
/// let result1 = precision_timestamp(&mut val1);
/// assert_eq!(result1, Ok(chrono::Utc.with_ymd_and_hms(2008, 10, 24, 0, 13, 29).unwrap() + chrono::Duration::milliseconds(913)));
/// ```
pub fn precision_timestamp(input: &mut &[u8]) -> winnow::PResult<chrono::DateTime<chrono::Utc>> {
    let checkpoint = input.checkpoint();
    // time in microseconds
    let ts = winnow::binary::be_u64.parse_next(input)?; 
    // time in seconds, time in nanoseconds
    let (ts, tns) = (ts / 1_000_000, (ts % 1_000_000) * 1_000);
    // convert to UTC
    match chrono::Utc.timestamp_opt(ts as i64, tns as u32) {
        chrono::LocalResult::Single(dt) => Ok(dt),
        chrono::LocalResult::None => Err(winnow::error::ErrMode::Backtrack(
            winnow::error::ContextError::new().add_context(
                input,
                &checkpoint,
                winnow::error::StrContext::Label("Invalid timestamp")
            )
        )),
        chrono::LocalResult::Ambiguous(_, _) => Err(winnow::error::ErrMode::Backtrack(winnow::error::ContextError::new())),
    }
}


// #[cfg(test)]
// mod tests {
//     use super::*;

//     // #[test]
//     // fn timestamp() {
//     //     let mut val: &[u8] = &(0x0004_59F4_A6AA_4AA8 as u64).to_be_bytes();
//     //     let val = precision_timestamp.parse_next(&mut val);
//     //     assert_eq!(val, Ok(chrono::Utc.with_ymd_and_hms(2008, 10, 24, 0, 13, 29).unwrap() + chrono::Duration::nanoseconds(913_000_000)));
//     // }
// }